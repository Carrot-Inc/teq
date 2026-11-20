// One class declared by two Java sources: the later one in the build's order is refused where it is declared (E161),
// the first one's statics object already defined, as the reference counts them.
// expect: B.java:1:1: error: J is already defined as object J in
object Use { val a: Int = J.a() }
