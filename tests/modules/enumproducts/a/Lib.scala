package enumproducts

// An enum read from the products keeps `DesugarEnums.enumScaffolding`'s model: its cases are made
// by its companion in order, an initialiser of its body running as each is (so the lookups made
// then answer as scalac's: `$values` not there yet, `valueOf` of a case not made), and `values`
// clones the `$values` kept once the last case exists; a mixed enum's `fromOrdinal` finds its
// value cases alone.
enum E:
  case A, B
  val n: Int = try E.values.length catch case _: Throwable => -1
  val v: Boolean = Option(E.valueOf("A")).isEmpty
  val f: String = try E.fromOrdinal(0).toString catch case _: NoSuchElementException => "missing"
enum M:
  case A
  case B(i: Int)
