// A default getter's expected type is the parameter's with the method's type parameters wildcards of their
// approximated bounds (`Namer.expectedDefaultArgType`, `ProtoTypes.wildApprox` with the parameter seen), the
// default's variables interpolated with the prototype's (`Inferencing.interpolateTypeVars`), and a conversion's
// result the getter's type (`typedAheadRhs(pt).tpe`): dotty's pos/i1103, pos/i1590 and pos/i8276.
class Sys[S]
class Foo[T <: Sys[T]] { val t: T = null.asInstanceOf[T]; def foo[A <: Sys[A]](x: A = t) = x }
case class W[T](seq: Option[Option[T]] = Option.empty)
object W { def apply[T] = new W[T]() }
object NOTHING
inline given [A] => Conversion[NOTHING.type, Option[A]] = _ => None
def h[A](p: Vector[A], o: Option[A] = NOTHING): Int = o.size
@main def run(): Unit = println(W[Int]().seq.size + h[String](Vector.empty))
