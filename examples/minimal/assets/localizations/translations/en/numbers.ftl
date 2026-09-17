# $value (String) - Number formatted for the active locale.
# $plural (String) - ICU plural category for that same visible value.
remaining = { $plural ->
    [one] { $value } item left
   *[other] { $value } items left
    }
# $count (Number) - Native Fluent numeric selector, independent of the Decimal adapter.
native = { $count ->
    [0] Empty
    [one] One item
   *[other] Several items
    }
