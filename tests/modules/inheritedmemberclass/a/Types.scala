package uva

// Member classes of a trait that an object inherits, as ultraviolet's `ShaderDSLTypes` gives
// `vec2` and `vec4` to `object syntax`: read from the products, the object's type members.
trait Types:
  final class vec2(val x: Int, val y: Int):
    def +(o: vec2): vec2 = new vec2(x + o.x, y + o.y)
    override def toString: String = s"vec2($x,$y)"
  final class vec4(val x: Int, val y: Int, val z: Int, val w: Int)

trait Ops extends Types:
  def dot(a: vec2, b: vec2): Int = a.x * b.x + a.y * b.y

object syntax extends Ops
