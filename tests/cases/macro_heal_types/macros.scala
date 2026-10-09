import scala.quoted.*

// A macro's reference to a value's type member takes the Type given in scope for it, imported
// from the value; an intersection matches a type pattern by its parts; the variables of a type
// pattern are bounded by the class's parameters; a constant type is no term reference.
trait Box[A <: AnyVal]

object M:
  inline def describe(): String = ${ describeImpl }

  trait Existential:
    type Underlying
    implicit val Underlying: Type[Underlying]

  def describeImpl(using q: Quotes): Expr[String] =
    import q.reflect.*
    val e: Existential = new Existential:
      type Underlying = String
      val Underlying = ConstantType(StringConstant("name")).asType.asInstanceOf[Type[String]]
    val healed =
      import e.{Underlying as U}
      Type.show[List[e.Underlying]]
    val inter = Type.of[Box[Int] & Serializable] match
      case '[Box[a]] => "box " + Type.show[a]
      case _ => "none"
    val bounded = Type.of[Box[Long]] match
      case '[Box[a]] => (TypeRepr.of[a] <:< TypeRepr.of[AnyVal]).toString
    val constant = Type.valueOfConstant[String](using e.Underlying.asInstanceOf[Type[String]])
    Expr(List(healed, inter, bounded, constant.toString).mkString(" / "))
