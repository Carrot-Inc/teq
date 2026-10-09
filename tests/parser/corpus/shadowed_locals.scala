// A local that shares its name with one of an enclosing scope is renamed in the output, so an
// outer val is not assigned to an inner constant of its name: pattern binders of a match that
// initialises a val, temporaries of nested calls with reordered named arguments, and locals of
// nested blocks and lambdas.

enum LoginType:
  case Normal(shelfType: Int)
  case Custom

final case class Id(value: Long)
final case class Inner(id: Id, name: String, other: Option[String])
final case class Outer(inner: Inner, id: Id, name: String)

object Shadow:
  def f(loginType: LoginType): Int =
    val shelfType = loginType match
      case LoginType.Normal(shelfType) => shelfType
      case LoginType.Custom            => 0
    shelfType

  def g: Outer =
    val product = Outer(
      name = "n",
      inner = Inner(other = Some("x"), id = Id(0), name = ""),
      id = Id(0)
    )
    product

  def h(xs: List[Option[Int]]): Int =
    val total = xs match
      case Some(total) :: rest =>
        val inner = rest match
          case Some(inner) :: _ => inner
          case _                => 0
        total + inner
      case _ => -1
    total

  def k(o: Option[Int]): String =
    val text =
      val text = o.map(text => text.toString).getOrElse("none")
      text + "!"
    text

@main def main(): Unit =
  println(Shadow.f(LoginType.Normal(3)))
  println(Shadow.f(LoginType.Custom))
  println(Shadow.g)
  println(Shadow.h(List(Some(1), Some(2), None)))
  println(Shadow.h(Nil))
  println(Shadow.k(Some(4)))
  println(Shadow.k(None))
