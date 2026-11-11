package rna

object Fmt:
  transparent inline def kind[T](x: Int): String =
    val a = x > 2
    if a then "yes " + x else "no " + x

object Use:
  def kinds(v: Int): String = Fmt.kind[String](v) + Fmt.kind[Int](v)
