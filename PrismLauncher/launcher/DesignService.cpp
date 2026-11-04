// SPDX-License-Identifier: GPL-3.0-only
#include "DesignService.h"
#include "Application.h"
#include "InstanceList.h"
#include "InstanceCreationTask.h"
#include "InstanceImportTask.h"
#include "InstanceCopyTask.h"
#include "icons/IconList.h"
#include <QBuffer>
#include "ResourceDownloadTask.h"
#include "minecraft/mod/ModFolderModel.h"
#include "tasks/SequentialTask.h"
#include <QFile>
#include "minecraft/VanillaInstanceCreationTask.h"
#include "minecraft/auth/AccountList.h"
#include "minecraft/auth/MinecraftAccount.h"
#include "minecraft/auth/AuthFlow.h"
#include "minecraft/PackProfile.h"
#include "launch/LaunchTask.h"
#include "launch/LogModel.h"
#include "meta/Index.h"
#include "meta/VersionList.h"
#include "settings/SettingsObject.h"
#include <QDateTime>
#include <QFileInfo>
#include <QWindow>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLocalSocket>
#include <QRegularExpression>
#include <QScopedValueRollback>
#include <QTimer>
#include <QUuid>
#include <stdexcept>

namespace {
void require(bool condition, const char* reason) { if (!condition) throw std::runtime_error(reason); }
const QStringList settingsKeys{"MinMemAlloc", "MaxMemAlloc", "JavaPath", "JvmArgs", "AutomaticJavaDownload", "AutomaticJavaSwitch"};
QJsonValue settingValue(SettingsObject* settings,const QString& key){
    const auto value=settings->get(key);
    if(key=="MinMemAlloc" || key=="MaxMemAlloc")return value.toInt();
    if(key=="AutomaticJavaDownload" || key=="AutomaticJavaSwitch")return value.toBool();
    return value.toString();
}
void validateSettings(const QJsonObject& changes, SettingsObject* settings) {
    for (auto it = changes.begin(); it != changes.end(); ++it) {
        require(settingsKeys.contains(it.key()), "Unsupported setting");
        if (it.key() == "MinMemAlloc" || it.key() == "MaxMemAlloc") {
            const auto value = it.value().toDouble(-1);
            require(value >= 256 && value <= 65536 && value == static_cast<int>(value), "Memory must be 256..65536 MiB");
        } else if (it.key() == "AutomaticJavaDownload" || it.key() == "AutomaticJavaSwitch") {
            require(it.value().isBool(), "Expected a boolean setting");
        } else {
            require(it.value().isString() && it.value().toString().size() <= 8192 && !it.value().toString().contains(QChar::Null), "Invalid text setting");
            if (it.key() == "JavaPath" && !it.value().toString().isEmpty())
                require(QFileInfo::exists(it.value().toString()), "Java executable does not exist");
        }
    }
    const auto minimum = changes.value("MinMemAlloc").toInt(settings->get("MinMemAlloc").toInt());
    const auto maximum = changes.value("MaxMemAlloc").toInt(settings->get("MaxMemAlloc").toInt());
    require(minimum <= maximum, "Minimum memory exceeds maximum memory");
}
BaseInstance* instanceFor(const QJsonObject& params) {
    auto instance = APPLICATION->instances()->getInstanceById(params.value("id").toString());
    require(instance != nullptr, "Instance not found");
    return instance;
}
void validateModrinthFile(const QJsonObject& file,const QString& extension){
    const auto name=file.value("filename").toString();
    const QUrl url(file.value("url").toString());
    require(!name.isEmpty() && name.size()<=200 && name.endsWith(extension,Qt::CaseInsensitive) && !name.contains(QRegularExpression("[\\\\/:<>\"|?*\\x00-\\x1f]")),"Invalid Modrinth filename");
    require(!QRegularExpression("^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\\.|$)",QRegularExpression::CaseInsensitiveOption).match(name).hasMatch(),"Reserved Windows filename");
    require(url.scheme()=="https" && url.host()=="cdn.modrinth.com" && url.userInfo().isEmpty() && url.port(-1)==-1 && url.path().startsWith("/data/"),"Invalid Modrinth download URL");
    require(QRegularExpression("^[a-fA-F0-9]{128}$").match(file.value("sha512").toString()).hasMatch(),"Invalid SHA-512");
}
}

DesignService::DesignService(const QString& name, QObject* parent) : QObject(parent) {
    m_server.setSocketOptions(QLocalServer::UserAccessOption);
    require(QRegularExpression("^graphite-[a-f0-9]{32}$").match(name).hasMatch(), "Invalid service name");
    require(m_server.listen(name), "Cannot start local service");
    connect(&m_server, &QLocalServer::newConnection, this, [this] {
        while (auto socket = m_server.nextPendingConnection()) {
            auto buffer = std::make_shared<QByteArray>();
            connect(socket, &QLocalSocket::disconnected, socket, &QObject::deleteLater);
            connect(socket, &QLocalSocket::readyRead, this, [this, socket, buffer] {
                buffer->append(socket->readAll());
                if (buffer->size() > 65536) { socket->abort(); return; }
                if (!buffer->contains('\n')) return;
                QJsonObject reply;
                try {
                    QJsonParseError parse;
                    auto document = QJsonDocument::fromJson(buffer->left(buffer->indexOf('\n')), &parse);
                    require(parse.error == QJsonParseError::NoError && document.isObject(), "Invalid request");
                    auto request = document.object();
                    reply.insert("result", handle(request.value("method").toString(), request.value("params").toObject()));
                    reply.insert("ok", true);
                } catch (const std::exception& error) {
                    reply = {{"ok", false}, {"error", QString::fromUtf8(error.what())}};
                }
                socket->write(QJsonDocument(reply).toJson(QJsonDocument::Compact) + '\n');
                socket->disconnectFromServer();
                buffer->clear();
            });
            QTimer::singleShot(60000, socket, [socket] { if (socket->isOpen()) socket->abort(); });
        }
    });
}

void DesignService::watch(Task* task, const std::shared_ptr<Job>& job) {
    connect(task, &Task::status, this, [job](const QString& value) { job->status = value; });
    connect(task, &Task::progress, this, [job](qint64 current, qint64 total) { job->current = current; job->total = total; });
    connect(task, &Task::stepProgress, this, [job](const TaskStepProgress& progress) {
        if (progress.total > 0) { job->current = progress.current; job->total = progress.total; }
        if (!progress.status.isEmpty()) job->status = progress.status;
    });
}

std::shared_ptr<DesignService::Job> DesignService::track(Task::Ptr task, const QString& label, const QString& instance) {
    auto job = std::make_shared<Job>();
    job->id = QUuid::createUuid().toString(QUuid::Id128);
    job->label = label; job->owner = task; job->task = task.get(); job->instance = instance;
    watch(task.get(), job);
    connect(task.get(), &Task::succeeded, this, [job] { job->state = job->cancelRequested?"cancelled":"succeeded"; job->auth = {}; });
    connect(task.get(), &Task::failed, this, [job](const QString& error) { job->state = "failed"; job->error = error; job->auth = {}; });
    connect(task.get(), &Task::aborted, this, [job] { job->state = "cancelled"; job->auth = {}; });
    connect(task.get(), &Task::finished, this, [this,job] { QTimer::singleShot(0,this,[job]{job->owner.reset();}); });
    m_jobs.append(job);
    for(int i=0;m_jobs.size()>16 && i<m_jobs.size();)if(m_jobs[i]->state!="running")m_jobs.removeAt(i);else ++i;
    return job;
}

QJsonObject DesignService::state() const {
    QJsonArray jobs, accounts;
    for (const auto& job : m_jobs) {
        QJsonObject value{{"id",job->id},{"label",job->label},{"state",job->state},{"error",job->error},{"status",job->status},{"current",double(job->current)},{"total",double(job->total)},{"instance",job->instance}};
        if (!job->auth.isEmpty()) value.insert("auth",job->auth);
        jobs.append(value);
    }
    auto list = APPLICATION->accounts();
    for (int i=0; i<list->count(); ++i) {
        auto account = list->at(i);
        accounts.append(QJsonObject{{"id",account->internalId()},{"name",account->displayName()},{"default",account == list->defaultAccount()},{"ownsMinecraft",account->ownsMinecraft()},{"hasProfile",account->hasProfile()}});
    }
    int visibleWindows=0;for(auto window:QGuiApplication::topLevelWindows())if(window->isVisible())++visibleWindows;
    return {{"jobs",jobs},{"accounts",accounts},{"visibleWindows",visibleWindows}};
}

QJsonObject DesignService::handle(const QString& method, const QJsonObject& params) {
    if (method == "state") return state();
    if(method=="icons.list"){
        QJsonArray icons;auto list=APPLICATION->icons();
        for(int i=0;i<list->rowCount() && icons.size()<150;++i){
            auto index=list->index(i,0);const auto key=list->data(index,Qt::UserRole).toString();
            if(key.startsWith("graphite-v1-"))continue; // Graphite renders this catalog from its shared sheets.
            QByteArray bytes;QBuffer buffer(&bytes);buffer.open(QIODevice::WriteOnly);list->getIcon(key).pixmap(48,48).save(&buffer,"PNG");
            icons.append(QJsonObject{{"key",key},{"name",list->data(index,Qt::DisplayRole).toString()},{"image",QString("data:image/png;base64,")+bytes.toBase64()}});
        }
        return {{"icons",icons}};
    }
    if(method=="instance.manage"){
        auto instance=instanceFor(params);const auto action=params.value("action").toString();
        require(!instance->isRunning(),"Stop Minecraft before changing the instance");
        for(const auto& job:m_jobs)require(job->state!="running" || job->instance!=instance->id(),"This instance has an active task");
        if(action=="rename"){
            auto name=params.value("name").toString().trimmed();require(!name.isEmpty() && name.size()<=100 && !name.contains(QRegularExpression("[\\x00-\\x1f]")),"Invalid instance name");
            instance->setName(name);instance->saveNow();return {{"ok",true}};
        }
        if(action=="icon"){
            auto key=params.value("icon").toString();require(APPLICATION->icons()->getIconIndex(key)>=0,"Icon not found");
            instance->setIconKey(key);instance->saveNow();return {{"ok",true}};
        }
        if(action=="duplicate"){
            auto task=new InstanceCopyTask(instance,InstanceCopyPrefs());task->setName(instance->name()+" (копия)");task->setIcon(instance->iconKey());task->setGroup(APPLICATION->instances()->getInstanceGroup(instance->id()));
            auto wrapped=Task::Ptr(APPLICATION->instances()->wrapInstanceTask(task));auto job=track(wrapped,"Copy instance",instance->id());QTimer::singleShot(0,wrapped.get(),&Task::start);return {{"job",job->id}};
        }
        if(action=="trash"){
            require(APPLICATION->instances()->trashInstance(instance->id()),"Could not move instance to trash");return {{"ok",true}};
        }
        throw std::runtime_error("Unsupported instance action");
    }
    if(method=="instance.log"){
        auto instance=instanceFor(params);auto task=instance->getLaunchTask();
        return {{"text",task?task->getLogModel()->toPlainText().right(200000):QString()}};
    }
    if(method=="instance.import"){
        for(const auto& job:m_jobs)require(job->state!="running" || (job->label!="Install Minecraft" && job->label!="Install Modrinth pack" && job->label!="Import instance"),"Another installation is running");
        QFileInfo file(params.value("path").toString());
        require(file.isAbsolute() && file.isFile() && !file.isSymLink() && (file.suffix().compare("zip",Qt::CaseInsensitive)==0 || file.suffix().compare("mrpack",Qt::CaseInsensitive)==0),"Invalid import archive");
        require(params.value("trusted").toBool(),"The selected archive has not been confirmed");
        auto name=params.value("name").toString().trimmed();require(!name.isEmpty()&&name.size()<=100&&!name.contains(QRegularExpression("[\\x00-\\x1f]")),"Invalid instance name");
        auto task=new InstanceImportTask(QUrl::fromLocalFile(file.absoluteFilePath()),true,nullptr);task->setName(name);task->setIcon("default");task->setGroup("");
        auto wrapped=Task::Ptr(APPLICATION->instances()->wrapInstanceTask(task));auto job=track(wrapped,"Import instance");QTimer::singleShot(0,wrapped.get(),&Task::start);return {{"job",job->id}};
    }
    if(method=="modrinth.pack"){
        require(!m_creating,"An installation is being prepared");
        for(const auto& job:m_jobs)require(job->state!="running" || (job->label!="Install Minecraft" && job->label!="Install Modrinth pack"),"Another installation is running");
        auto file=params.value("file").toObject();validateModrinthFile(file,".mrpack");
        const auto name=params.value("name").toString().trimmed();
        require(!name.isEmpty() && name.size()<=100 && !name.contains(QRegularExpression("[\\x00-\\x1f]")),"Invalid instance name");
        const auto project=params.value("project").toString(),version=params.value("version").toString();
        require(QRegularExpression("^[a-zA-Z0-9]{1,32}$").match(project).hasMatch() && QRegularExpression("^[a-zA-Z0-9]{1,32}$").match(version).hasMatch(),"Invalid Modrinth identifiers");
        QMap<QString,QString> extra{{"pack_id",project},{"pack_version_id",version},{"graphite_optional_mods",params.value("optional").toBool(true)?"true":"false"}};
        auto creation=new InstanceImportTask(QUrl(file.value("url").toString()),true,nullptr,std::move(extra));
        creation->setArchiveSha512(file.value("sha512").toString());creation->setName(name);creation->setIcon("modrinth");creation->setGroup("");
        auto wrapped=Task::Ptr(APPLICATION->instances()->wrapInstanceTask(creation));
        auto job=track(wrapped,"Install Modrinth pack");QTimer::singleShot(0,wrapped.get(),&Task::start);
        return {{"job",job->id}};
    }
    if(method=="modrinth.mods"){
        auto instance=dynamic_cast<MinecraftInstance*>(instanceFor(params));
        require(instance && !instance->isRunning(),"Stop Minecraft before installing mods");
        for(const auto& job:m_jobs)require(job->state!="running" || job->instance!=instance->id(),"This instance has an active task");
        auto profile=instance->getPackProfile();
        auto loaded=profile->loadComponents();
        if(!loaded)throw std::runtime_error(loaded.error.toStdString());
        QString loader;
        for(const auto& pair:QList<QPair<QString,QString>>{{"net.fabricmc.fabric-loader","fabric"},{"net.minecraftforge","forge"},{"net.neoforged","neoforge"},{"org.quiltmc.quilt-loader","quilt"}})
            if(!profile->getComponentVersion(pair.first).isEmpty())loader=pair.second;
        require(!loader.isEmpty(),"Install a mod loader before adding mods");
        const auto minecraft=profile->getComponentVersion("net.minecraft");
        auto files=params.value("files").toArray();require(!files.isEmpty() && files.size()<=64,"Invalid mod installation plan");
        require(!QFileInfo(instance->modsRoot()).isSymLink(),"Linked mods directories are not supported");
        auto sequence=makeShared<SequentialTask>();
        for(const auto& value:files){
            auto file=value.toObject();validateModrinthFile(file,".jar");
            require(file.value("gameVersions").toArray().contains(minecraft) && file.value("loaders").toArray().contains(loader),"Mod is incompatible with this instance");
            auto pack=std::make_shared<ModPlatform::IndexedPack>();pack->addonId=file.value("project").toString();pack->name=file.value("title").toString();pack->provider=ModPlatform::ResourceProvider::MODRINTH;
            pack->slug=file.value("slug").toString(file.value("project").toString());
            require(QRegularExpression("^[a-zA-Z0-9_-]{1,100}$").match(pack->slug).hasMatch(),"Invalid mod slug");
            ModPlatform::IndexedVersion version;version.addonId=pack->addonId;version.fileId=file.value("version").toString();version.version=file.value("versionName").toString();version.version_number=version.version;version.fileName=file.value("filename").toString();version.downloadUrl=file.value("url").toString();version.hash_type="sha512";version.hash=file.value("sha512").toString();version.mcVersion={minecraft};version.loaders=ModPlatform::getModLoaderFromString(loader);
            const auto target=QDir(instance->modsRoot()).filePath(version.fileName);
            require(!QFileInfo(target).isSymLink(),"Linked mod files are not supported");
            sequence->addTask(makeShared<ResourceDownloadTask>(pack,version,instance->loaderModList(),true,"standalone"));
        }
        auto job=track(sequence,"Install Modrinth mods",instance->id());QTimer::singleShot(0,sequence.get(),&Task::start);
        return {{"job",job->id}};
    }
    if (method == "settings.get" || method == "settings.set") {
        auto settings = APPLICATION->settings();
        if (method == "settings.set") {
            validateSettings(params,settings);
            for (auto it=params.begin();it!=params.end();++it) settings->set(it.key(),it.value().toVariant());
        }
        QJsonObject result;
        for (const auto& key : settingsKeys) result.insert(key,settingValue(settings,key));
        return result;
    }
    if (method == "instance.get" || method == "instance.set") {
        auto instance = instanceFor(params);
        auto settings = instance->settings();
        if (method == "instance.set") {
            auto changes=params.value("settings").toObject();
            require(!changes.contains("AutomaticJavaDownload") && !changes.contains("AutomaticJavaSwitch"),"Automatic Java is a global setting");
            validateSettings(changes,settings);
            if(changes.contains("MinMemAlloc") || changes.contains("MaxMemAlloc"))settings->set("OverrideMemory",true);
            if(changes.contains("JvmArgs"))settings->set("OverrideJavaArgs",true);
            if(changes.contains("JavaPath"))settings->set("OverrideJavaLocation",!changes.value("JavaPath").toString().isEmpty());
            for(auto it=changes.begin();it!=changes.end();++it)settings->set(it.key(),it.value().toVariant());
        }
        QJsonObject result{{"id",instance->id()},{"name",instance->name()},{"icon",instance->iconKey()},{"managedType",instance->getManagedPackType()},{"managedId",instance->getManagedPackID()},{"managedVersion",instance->getManagedPackVersionID()}};
        QJsonObject values;
        for(const auto& key:settingsKeys)if(key!="AutomaticJavaDownload" && key!="AutomaticJavaSwitch")values.insert(key,settingValue(settings,key));
        result.insert("settings",values);
        return result;
    }
    if (method == "cancel") {
        for (const auto& job : m_jobs) if(job->id == params.value("id").toString() && job->state == "running") {
            require(job->task && (job->task->canAbort() || job->label=="Microsoft"),"Task cannot be cancelled at this stage");
            job->cancelRequested=true;
            if(!job->task->abort()){job->cancelRequested=false;throw std::runtime_error("Task refused cancellation");}
            return {{"ok",true}};
        }
        throw std::runtime_error("Active task not found");
    }
    if (method == "accounts.default" || method == "accounts.remove") {
        for(const auto& job:m_jobs)require(job->state!="running" || job->instance.isEmpty(),"Account is in use by Minecraft");
        auto list=APPLICATION->accounts();
        for(int i=0;i<list->count();++i)if(list->at(i)->internalId()==params.value("id").toString()) {
            require(!list->at(i)->isActive(),"Account is in use");
            if(method=="accounts.default")list->setDefaultAccount(list->at(i));
            else list->removeAccount(list->index(i,0));
            return state();
        }
        throw std::runtime_error("Account not found");
    }
    if (method == "login") {
        for(const auto& job:m_jobs)require(job->state!="running" || job->label!="Microsoft", "Login is already running");
        auto account=MinecraftAccount::createBlankMSA();
        auto flow=account->login(true);
        auto job=track(flow,"Microsoft");
        connect(flow.get(),&AuthFlow::authorizeWithBrowserWithExtra,this,[job](const QString& url,const QString& code,int expires) {
            job->auth={{"url",url},{"code",code},{"expires",QDateTime::currentMSecsSinceEpoch()+qint64(expires)*1000}};
        });
        connect(flow.get(),&AuthFlow::authorizeWithBrowser,this,[job](const QUrl& url){job->auth={{"url",url.toString()}};});
        connect(flow.get(),&Task::succeeded,this,[account] {
            APPLICATION->accounts()->addAccount(account);
            APPLICATION->accounts()->setDefaultAccount(account);
        });
        // Retain the account while AuthFlow keeps a pointer to its data.
        connect(flow.get(),&Task::finished,this,[account] { Q_UNUSED(account); });
        QTimer::singleShot(0,flow.get(),&Task::start);
        return {{"job",job->id}};
    }
    if (method == "create") {
        require(!m_creating,"An installation is being prepared");
        QScopedValueRollback<bool> preparing(m_creating,true);
        const auto name=params.value("name").toString().trimmed();
        const auto version=params.value("version").toString();
        require(!name.isEmpty() && name.size()<=100 && !name.contains(QRegularExpression("[\\x00-\\x1f]")),"Invalid instance name");
        for(const auto& job:m_jobs)require(job->state!="running" || (job->label!="Install Minecraft" && job->label!="Install Modrinth pack"), "Another installation is running");
        auto list=APPLICATION->metadataIndex()->get("net.minecraft");
        list->waitToLoad();
        require(list->hasVersion(version),"Minecraft version not found");
        auto minecraft=list->getVersion(version);
        VanillaCreationTask* creation=nullptr;
        const auto loader=params.value("loader").toString();
        if(loader.isEmpty() || loader=="vanilla")creation=new VanillaCreationTask(minecraft);
        else {
            require(QStringList{"net.fabricmc.fabric-loader","net.minecraftforge","net.neoforged","org.quiltmc.quilt-loader"}.contains(loader),"Unsupported mod loader");
            auto loaders=APPLICATION->metadataIndex()->get(loader);loaders->waitToLoad();
            const auto loaderVersion=params.value("loaderVersion").toString();
            require(loaders->hasVersion(loaderVersion),"Mod loader version not found");
            creation=new VanillaCreationTask(minecraft,loader,loaders->getVersion(loaderVersion));
        }
        creation->setName(name);creation->setIcon("grass");creation->setGroup("");
        auto wrapped=Task::Ptr(APPLICATION->instances()->wrapInstanceTask(creation));
        auto job=track(wrapped,"Install Minecraft");
        QTimer::singleShot(0,wrapped.get(),&Task::start);
        return {{"job",job->id}};
    }
    if (method == "launch") {
        MinecraftTarget::Ptr target;
        if(params.contains("server")){
            require(params.value("server").isString(),"Invalid server address");
            const auto address=params.value("server").toString();const QUrl url("minecraft://"+address);
            require(!address.isEmpty() && address.size()<=255 && !address.contains(QRegularExpression("[\\s/\\\\?#@]")) && url.isValid() && !url.host().isEmpty() && url.port(25565)>0 && url.port(25565)<=65535 && url.userInfo().isEmpty(),"Invalid server address");
            target=std::make_shared<MinecraftTarget>();target->address=url.host();target->port=static_cast<quint16>(url.port(25565));
        }
        auto instance=instanceFor(params);
        for(const auto& active:m_jobs)require(active->state!="running" || active->instance!=instance->id(),"This instance is already launching");
        require(instance->canLaunch() && !instance->isRunning(),"Instance cannot be launched now");
        if (params.contains("twitchland") || params.contains("offline")) {
            const bool offline=params.contains("offline");
            require(!(offline && params.contains("twitchland")),"Choose one account type");
            require(params.value(offline?"offline":"twitchland").isObject(), "Invalid account session");
            const auto auth=params.value(offline?"offline":"twitchland").toObject();
            const auto name=auth.value("name").toString();
            auto uuid=auth.value("uuid").toString();
            const auto token=auth.value("token").toString();
            require(QRegularExpression("^[A-Za-z0-9_]{1,16}$").match(name).hasMatch(),"Invalid player name");
            uuid.remove('-');
            if(!offline){
                require(QRegularExpression("^[a-fA-F0-9]{32}$").match(uuid).hasMatch(),"Invalid TwitchLand profile ID");
                require(token.size()>0 && token.size()<=16384 && !token.contains(QRegularExpression("[\\x00-\\x20\\x7f]")),"Invalid TwitchLand token");
            }
            auto session=std::make_shared<AuthSession>();
            if(offline){
                session->MakeOffline(name);
                session->uuid=MinecraftAccount::uuidFromUsername(name).toString(QUuid::Id128);
                // Local identity still needs the normal game-file download pipeline on first launch.
                session->user_type="offline";session->launchMode=LaunchMode::Normal;
            }else{
                session->player_name=name;session->uuid=uuid;session->access_token=token;
                session->session="token:"+token+":"+uuid;session->user_type="msa";
                session->launchMode=LaunchMode::Normal;session->twitchLand=true;
            }
            auto launch=instance->createLaunchTask(session,target);
            require(launch!=nullptr,"Cannot create launch task");
            auto job=std::make_shared<Job>();job->id=QUuid::createUuid().toString(QUuid::Id128);
            job->label="Launch Minecraft";job->instance=instance->id();job->task=launch;m_jobs.append(job);watch(launch,job);
            for(int i=0;m_jobs.size()>16 && i<m_jobs.size();)if(m_jobs[i]->state!="running")m_jobs.removeAt(i);else ++i;
            connect(launch,&LaunchTask::readyForLaunch,launch,&LaunchTask::proceed);
            connect(launch,&LaunchTask::requestProgress,this,[this,launch,job](Task* task){watch(task,job);launch->proceed();});
            connect(launch,&Task::succeeded,this,[job]{job->state=job->cancelRequested?"cancelled":"succeeded";});
            connect(launch,&Task::failed,this,[job](const QString& error){job->state=job->cancelRequested?"cancelled":"failed";if(!job->cancelRequested)job->error=error;});
            connect(launch,&Task::aborted,this,[job]{job->state="cancelled";});
            QTimer::singleShot(0,launch,&Task::start);
            return {{"job",job->id}};
        }
        auto account=APPLICATION->accounts()->defaultAccount();
        require(bool(account),"AUTH_REQUIRED");
        require(account->ownsMinecraft() && account->hasProfile(),"Minecraft Java ownership or profile is missing");
        auto refresh=account->refresh();
        auto job=track(refresh,"Launch Minecraft",instance->id());
        QPointer<BaseInstance> guardedInstance(instance);
        connect(refresh.get(),&Task::succeeded,this,[this,account,guardedInstance,job,target] {
            auto instance=guardedInstance.data();
            if(!instance){job->state="failed";job->error="Instance no longer exists";return;}
            if(account->accountState()!=AccountState::Online || !account->ownsMinecraft()){job->state="failed";job->error="Sign in again to launch Minecraft";return;}
            auto session=std::make_shared<AuthSession>();account->fillSession(session);session->launchMode=LaunchMode::Normal;
            auto launch=instance->createLaunchTask(session,target);
            if(!launch){job->state="failed";job->error="Cannot create launch task";return;}
            job->task=launch;job->state="running";watch(launch,job);
            connect(launch,&LaunchTask::readyForLaunch,launch,&LaunchTask::proceed);
            connect(launch,&LaunchTask::requestProgress,this,[this,launch,job](Task* task){watch(task,job);launch->proceed();});
            connect(launch,&Task::succeeded,this,[job]{job->state=job->cancelRequested?"cancelled":"succeeded";});
            connect(launch,&Task::failed,this,[job](const QString& error){job->state=job->cancelRequested?"cancelled":"failed";if(!job->cancelRequested)job->error=error;});
            connect(launch,&Task::aborted,this,[job]{job->state="cancelled";});
            QTimer::singleShot(0,launch,&Task::start);
        });
        if(!refresh->isRunning())QTimer::singleShot(0,refresh.get(),&Task::start);
        return {{"job",job->id}};
    }
    if (method == "shutdown") {
        bool active=false;for(const auto& job:m_jobs)active |= job->state=="running";
        if(active && params.value("detach").toBool()) {
            auto timer=new QTimer(this);timer->setInterval(1000);
            connect(timer,&QTimer::timeout,this,[this,timer]{for(const auto& job:m_jobs)if(job->state=="running")return;timer->stop();APPLICATION->quit();});
            timer->start();
        } else {
            require(!active,"Tasks are still running");
            QTimer::singleShot(50,APPLICATION,[]{APPLICATION->quit();});
        }
        return {{"ok",true}};
    }
    throw std::runtime_error("Unsupported service method");
}
