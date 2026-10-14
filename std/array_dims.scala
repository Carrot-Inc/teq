package scala

// dotty's `ArrayConstructors`: `Array.ofDim` of an element type whose erasure is known makes its arrays
// as `java.lang.reflect.Array.newInstance` does over every dimension, which refuses a negative one before
// it makes any (`NegativeArraySizeException`); a generic `ofDim` (scala-library's) makes the outer array
// first and an inner one only where there is an element for it. The typer calls these in such a call's
// place, the receiver first, its pickle writing the call as written (`Form::Member`).
def ofDimKnown[T](array: Any, n1: Int, n2: Int, ct: scala.reflect.ClassTag[T]): Array[Array[T]] =
  refuseNegative(n1, n2)
  Array.ofDim[T](n1, n2)(using ct)
def ofDimKnown[T](array: Any, n1: Int, n2: Int, n3: Int, ct: scala.reflect.ClassTag[T]): Array[Array[Array[T]]] =
  refuseNegative(n1, n2, n3)
  Array.ofDim[T](n1, n2, n3)(using ct)
def ofDimKnown[T](array: Any, n1: Int, n2: Int, n3: Int, n4: Int, ct: scala.reflect.ClassTag[T]): Array[Array[Array[Array[T]]]] =
  refuseNegative(n1, n2, n3, n4)
  Array.ofDim[T](n1, n2, n3, n4)(using ct)
def ofDimKnown[T](array: Any, n1: Int, n2: Int, n3: Int, n4: Int, n5: Int, ct: scala.reflect.ClassTag[T]): Array[Array[Array[Array[Array[T]]]]] =
  refuseNegative(n1, n2, n3, n4, n5)
  Array.ofDim[T](n1, n2, n3, n4, n5)(using ct)

private def refuseNegative(dims: Int*): Unit =
  for d <- dims do
    if d < 0 then throw new NegativeArraySizeException(d.toString)
