remaining = { $plural ->
    [one] Остался { $value } предмет
    [few] Осталось { $value } предмета
    [many] Осталось { $value } предметов
   *[other] Осталось { $value } предмета
    }
native = { $count ->
    [0] Пусто
    [one] Один предмет
    [few] Несколько предметов
    [many] Много предметов
   *[other] Другие предметы
    }
