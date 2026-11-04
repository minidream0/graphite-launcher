// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <QObject>
#include <QJsonObject>
#include <QLocalServer>
#include <QList>
#include <QPointer>
#include "tasks/Task.h"

class DesignService final : public QObject {
 public:
    explicit DesignService(const QString& name, QObject* parent);
 private:
    struct Job {
        QString id, label, state = "running", error, instance;
        Task::Ptr owner;
        QPointer<Task> task;
        QJsonObject auth;
        qint64 current = 0, total = 0;
        QString status;
        bool cancelRequested=false;
    };
    QLocalServer m_server;
    QList<std::shared_ptr<Job>> m_jobs;
    bool m_creating=false;
    QJsonObject handle(const QString& method, const QJsonObject& params);
    QJsonObject state() const;
    std::shared_ptr<Job> track(Task::Ptr task, const QString& label, const QString& instance = {});
    void watch(Task* task, const std::shared_ptr<Job>& job);
};
