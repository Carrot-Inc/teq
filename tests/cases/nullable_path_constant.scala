// `stripNull` widens and dealiases at each step: `n.type` of a `Byte | Null` value widens to `Byte | Null`, whose `Null`
// goes, so `Byte | n.type` has the class bound `Byte` and the constant converts, as scalac 3.8.4 accepts it (the review
// of `70fa37159`, item 3; `n.type` of a `Null` value stays itself, `isNullType` reading no widening).
def id[T](x: T): T = x
val n: Byte | Null = null
val b: Byte | n.type = id[65](65)

@main def run(): Unit = println(b)
