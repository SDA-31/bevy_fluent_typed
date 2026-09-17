remaining = { $plural ->
    [one] Queda { $value } elemento
   *[other] Quedan { $value } elementos
    }
native = { $count ->
    [0] Vacío
    [one] Un elemento
   *[other] Varios elementos
    }
