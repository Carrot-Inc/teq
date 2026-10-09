package fpt

object A:
  inline def test(x: Any): Boolean = x.isInstanceOf[Int]
