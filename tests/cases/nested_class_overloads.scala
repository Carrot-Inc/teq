// Overloads over classes nested in a trait, through the path a subtrait sees them by (ultraviolet's
// `abs(vec2)`, `abs(vec3)`): each erases to its class, as dotty's `TypeErasure` erases a path's class,
// so the alternatives are distinct and each call reaches its own. (The instances are made with `new`: a bare
// `V2(..)` in `Ops` takes `Types.this.V2` for `Ops.this.V2` on master as here, a case class nested in a trait
// having no companion object.)
trait Types:
  final case class V2(x: Int, y: Int)
  final case class V3(x: Int, y: Int, z: Int)
trait Ops extends Types:
  def abs(x: Int): Int = math.abs(x)
  def abs(v: V2): V2 = new V2(abs(v.x), abs(v.y))
  def abs(v: V3): V3 = new V3(abs(v.x), abs(v.y), abs(v.z))
object syntax extends Ops
import syntax.*

// zio's `Trace`, an abstract type member read through a stable path and bounded by `AnyRef`: an
// overload's parameter of it erases to the bound, as `TypeErasure` erases an abstract type.
trait Tracer:
  type Type <: AnyRef
  def empty: Type
object Tracer:
  val instance: Tracer = new Tracer:
    type Type = String
    def empty = "traced"
type Trace = Tracer.instance.Type
object Clock:
  def currentTime(unit: => Int)(implicit trace: Trace): String = s"$unit $trace"
  def currentTime(unit: Int, at: Long): String = s"$unit at $at"

@main def run(): Unit =
  println(abs(V2(-1, 2)))
  println(abs(V3(-1, 2, -3)))
  println(abs(-4))
  given Trace = Tracer.instance.empty
  println(Clock.currentTime(3))
  println(Clock.currentTime(4, 5L))
