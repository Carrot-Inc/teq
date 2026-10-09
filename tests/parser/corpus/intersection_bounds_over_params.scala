// Bounds over type parameters meet on an intersection's member whether or not they can be
// compared (`>: A <: B` for `Lo[A] & Hi[B]`), as scalac's `&` of two `TypeBounds`: only ground
// bounds that conflict keep the first side's.
trait Lo[A] { type T >: A }
trait Hi[B] { type T <: B }

def f[A, B](x: Lo[A] & Hi[B])(a: A): B = { val t: x.T = a; t }

@main def Main(): Unit =
  val x = new Lo[String] with Hi[CharSequence] { type T = String }
  println(f(x)("s").length)
