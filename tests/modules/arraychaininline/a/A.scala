package lib
inline def pairLength(x: Any): Int = x.asInstanceOf[Array[Int *: String *: EmptyTuple]].length
inline def oneLength(inline x: Any): Int = x.asInstanceOf[Array[Int *: EmptyTuple]].length
