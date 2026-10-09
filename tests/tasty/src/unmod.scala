package fix.unmod

import scala.scalajs.js

// Library bodies of the shapes the application's jars have on JavaScript: scala-library's
// packages the std writes as objects, Scala.js's facades, an implicit clause passed explicitly.
object UmBodies:
  def fail(s: String): Nothing = sys.error(s)
  def attempt(s: String): String = try fail(s) catch case e: RuntimeException => "caught " + e.getMessage
  def revConcat(a: List[Int], b: List[Int]): List[Int] = a reverse_::: b
  def hasKey(o: js.Object, k: String): Boolean = js.Object.hasProperty(o, k)
  def keysOf(o: js.Object): List[String] = js.Object.properties(o).toList.sorted
  def describe(s: js.Symbol): String = js.Symbol.keyFor(s).getOrElse("?")
  def define(target: js.Object, key: String, v: Int): Unit =
    val d = js.Dynamic.literal(value = v).asInstanceOf[js.PropertyDescriptor]
    d.writable = true
    d.configurable = true
    js.Object.defineProperty(target, key, d)
  def decoded(name: String): String = scala.reflect.NameTransformer.decode(name)

object UmSyntax:
  implicit final class UmReuseOps[A](private val self: A) extends AnyVal:
    def ~=~(a: A)(implicit r: UmReuse[A]): Boolean = r.test(self, a)

final class UmReuse[A](val test: (A, A) => Boolean)

object UmReuse:
  import UmSyntax.*
  def apply[A](f: (A, A) => Boolean): UmReuse[A] = new UmReuse(f)
  given ints: UmReuse[Int] = apply(_ == _)
  given strs: UmReuse[String] = apply(_ == _)
  def pair[A: UmReuse, B: UmReuse]: UmReuse[(A, B)] = apply((x, y) => (x._1 ~=~ y._1) && (x._2 ~=~ y._2))

@js.native
trait UmMethod extends js.Object with js.PropertyDescriptor:
  val key: String = js.native

object UmDefine:
  def method(key: String, v: Any): UmMethod = js.Dynamic.literal(key = key, value = v.asInstanceOf[js.Any]).asInstanceOf[UmMethod]
  def defineAll(target: js.Object, props: js.Array[UmMethod]): Unit =
    props.foreach { p =>
      p.configurable = true
      if js.Object.hasProperty(p, "value") then p.writable = true
      js.Object.defineProperty(target, p.key, p)
    }

object UmNested:
  @js.native
  private trait Prop extends js.Object with js.PropertyDescriptor:
    val key: String = js.native
  private def Prop(key: String, v: Any): Prop = js.Dynamic.literal(key = key, value = v.asInstanceOf[js.Any]).asInstanceOf[Prop]
  def define(target: js.Object, key: String, v: Any): Unit =
    val p = Prop(key, v)
    p.configurable = true
    p.writable = true
    js.Object.defineProperty(target, p.key, p)

// A type lambda written as the projection of a refinement (`({ type F[A] = .. })#F`), in the
// arguments of a class instantiated in a body and of the alias the body's expected type names
// (scalajs-react's `Custom_SubsequentSteps`).
final class UmSub[I, F[_]](val s: String)

trait UmStep[I, F[_]]:
  type Next[H]
  def next[H](s: String): Next[H]

object UmStep:
  type To[I, F[_], N[_]] = UmStep[I, F] { type Next[A] = N[A] }
  trait Dsl[I, H1]:
    type Next[H2] = UmSub[I, ({ type F[A] = (I, H1, H2) => A })#F]
  def atStep1[I, H1]: To[I, ({ type F[A] = (I, H1) => A })#F, Dsl[I, H1]#Next] =
    new UmStep[I, ({ type F[A] = (I, H1) => A })#F]:
      type Next[H2] = Dsl[I, H1]#Next[H2]
      def next[H2](s: String): Next[H2] = new UmSub[I, ({ type F[A] = (I, H1, H2) => A })#F](s)

// A comparison in a library body of a case class without a `CanEqual`, which a program compiled
// under strict equality calls.
final case class UmPt(x: Int, y: Int)

object UmEq:
  def same(a: UmPt, b: UmPt): Boolean = a == b
