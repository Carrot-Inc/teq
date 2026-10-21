// `isNullType` is `isRef(defn.NullClass)`, which reads an annotated type's parent and a refinement's: `stripNull` drops
// `Null @RuntimeChecked` and `Null { type T = Int }` from the union, so the class bound is `Byte` (`Short`, `Char`) and
// the constant converts, as scalac 3.8.4 accepts it (the review of `6c316c160`, finding 1's NullableAnnot).
def id[T](x: T): T = x
val a: Byte | (Null @scala.annotation.internal.RuntimeChecked) = id[65](65)
val b: Short | (Null @scala.annotation.internal.RuntimeChecked) = id[66](66)
val c: (Null @scala.annotation.internal.RuntimeChecked) | Char = id[67](67)
val d: Byte | (Null { type T = Int }) = id[68](68)

@main def run(): Unit = println(s"$a $b ${c == 'C'} $d")
