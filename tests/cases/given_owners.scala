// The owners of a parameterless given with a class in its body: a package object, objects, a class, a trait,
// a method, companions, givens with parameters, anonymous, alias, exported and private givens, nested givens.
package cases
trait T { def value: Int }
package object pp {
  given G: T with { def value = 21; class K { def get = value } }
}
object Named { given G: T with { def value = 21; class K { def get = value } } }
class Host { object O { given G: T with { def value = 21; class K { def get = value } } }; def run = new O.G.K().get }
trait Tr { given G: T with { def value = 21; class K { def get = value } }; def run = new G.K().get }
object TrImpl extends Tr
object Method { def run: Int = { given G: T with { def value = 21; class K { def get = value } }; new G.K().get } }
enum E { case A }
object E { given G: T with { def value = 21; class K { def get = value } } }
class V(val x: Int) extends AnyVal
object V { given G: T with { def value = 21; class K { def get = value } } }
object Params { given G(using n: Int): T with { def value = n; class K { def get = value }; def run = new K().get }; def run = G(using 21).run }
object Types { given G[A]: T with { def value = 21; class K { def get = value }; def run = new K().get }; def run = G[Int].run }
object Anon { given T with { def value = 21; class K { def get = value } }; def run = new given_T.K().get }
object Alias { class Impl extends T { def value = 21; class K { def get = value } }; given G: Impl = new Impl; def run = new G.K().get }
object Export { export Named.G; def run = new G.K().get }
object Private { private given G: T with { def value = 21; class K { def get = value } }; def run = new G.K().get }
object NestedParams { given Outer(using n: Int): T with { def value = n; given G: T with { def value = n; class K { def get = value } }; def run = new G.K().get }; def run = Outer(using 21).run }
object Main { def main(args: Array[String]): Unit = {
 println(new pp.G.K().get); println(new Named.G.K().get); println(new Host().run); println(TrImpl.run); try println(Method.run) catch { case _: Throwable => println("ERR:method") }
 println(new E.G.K().get); println(new V.G.K().get); println(Params.run); println(Types.run); println(Anon.run); println(Alias.run); println(Export.run); println(Private.run); println(NestedParams.run)
} }
