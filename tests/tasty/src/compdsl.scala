// A body that passes a given its companion object inherits from a trait (scalajs-react's
// `RouteB.Composition.Ato_`): the reference names the object the given is reached through.
package fix.shapes

object CompDsl:
  final class RouteB[A](val name: String):
    def ~[B](next: RouteB[B])(implicit c: RouteB.Composition[A, B]): RouteB[c.C] = new RouteB(s"$name~${next.name}")
    def /[B](next: RouteB[B])(implicit c: RouteB.Composition[A, B]): RouteB[c.C] = this ~ RouteB.slash ~ next
  object RouteB:
    val slash: RouteB[Unit] = new RouteB("/")
    abstract class Composition[A, B]:
      type C
    trait Composition_PriMed:
      implicit def Ato_[A]: Composition.Aux[A, Unit, A] = Composition[A, Unit, A]()
    object Composition extends Composition_PriMed:
      type Aux[A, B, O] = Composition[A, B] { type C = O }
      def apply[A, B, O](): Aux[A, B, O] = new Composition[A, B] { type C = O }
