export function diagnose(text){
 const value=String(text||'');
 if(/UnsupportedClassVersionError|class file version.*(?:recognizes|up to)|requires (?:a newer version of )?java|requires java \d+/i.test(value))return 'Вероятно, версия Java не подходит для этой сборки. Проверь Java в настройках сборки.';
 if(/OutOfMemoryError|Could not reserve enough space|insufficient memory for the Java Runtime|unable to create native thread/i.test(value))return 'Похоже, Java не хватает памяти. Закрой лишние программы или измени объём памяти в настройках.';
 if(/requires .*? (?:which is missing|is missing|not installed)|missing (?:required )?dependenc|depends on .*? which is missing|ModResolutionException/i.test(value))return 'Похоже, отсутствует необходимая зависимость мода. Её название обычно указано ниже в журнале.';
 if(/Mixin apply failed|MixinTransformerError|incompatible mod set|mod.*(?:conflicts with|is incompatible with)/i.test(value))return 'Похоже, моды конфликтуют или несовместимы с версией игры. Подробности есть в журнале.';
 return '';
}
