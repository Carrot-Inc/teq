// A case class nested in a trait whose companion declares an `unapply` of its own, over a type
// member of the trait: a pattern in another trait of the cake, which has the first in its self
// type, selects among that `unapply` and the synthetic one, each seen from the enclosing `this`.
trait Defs:
  type Type[A]
  type Expr[A]
  def exprOf[A](t: Type[A]): Option[Expr[A]]

trait Singles { this: Defs =>
  final protected case class Single[A](value: Expr[A])
  protected object Single:
    def unapply[A](A: Type[A]): Option[Expr[A]] = exprOf(A)
}

trait Rules { this: Derivation =>
  def expand[To](t: Type[To]): String = t match
    case Single(e) => "single " + e
    case _ => "none"

  def held[To](t: Type[To]): String = exprOf(t).map(e => Single[To](e)) match
    case Some(s) => s match
      case Single(v) => "held " + v
    case None => "nothing held"
}

trait Derivation extends Defs with Singles with Rules

object Impl extends Derivation:
  type Type[A] = Class[A]
  type Expr[A] = String
  def exprOf[A](t: Type[A]): Option[Expr[A]] = if t == classOf[String] then Some("str") else None

@main def run(): Unit =
  println(Impl.expand(classOf[String]))
  println(Impl.expand(classOf[Int]))
  println(Impl.held(classOf[String]))
  println(Impl.held(classOf[Int]))
