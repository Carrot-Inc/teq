// An array of `Nothing` and one of `Null` are arrays like any other: the first holds no
// element it could give, the second holds `null`.
def lengths[T](xs: Array[T]): Int = xs.length

@main def run(): Unit =
  println(new Array[Nothing](0).length)
  println(new Array[Null](1).length)
  val none = new Array[Nothing](0)
  val nulls = new Array[Null](2)
  println(none.length.toString + " " + nulls.length + " " + (nulls(0) == null) + " " + lengths(none) + lengths(nulls))
  println(none.toList.toString + " " + nulls.toList + " " + none.isEmpty + " " + nulls.map(x => x == null).mkString(","))
  val any: Array[? <: Any] = none
  val more: Array[? <: AnyRef] = nulls
  println(any.length.toString + " " + more.length + " " + none.clone().length + " " + nulls.clone().length)
