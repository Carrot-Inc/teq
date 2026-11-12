// The parameter of a lambda whose expected type leaves it open is typed from the callee its body applies to
// it: a constructor proxy, a method of the package, of an object, of the block, a function value, a method
// with a using clause, with a default parameter, and one taking another argument beside it; each written
// with a placeholder and with a named parameter.
final case class Tag(name: String)
object Tags:
  def of(name: String): Tag = Tag(name + "!")
def top(name: String): Tag = Tag(name + "?")
val value: String => Tag = s => Tag(s + ".")
def withUsing(name: String)(using suffix: Int): Tag = Tag(name + suffix)
def withDefault(name: String, times: Int = 2): Tag = Tag(name * times)
def beside(name: String, n: Int): Tag = Tag(name + n)

def on[B, R](f: B => R): B => R = f

@main def run(): Unit =
  given Int = 7
  def local(name: String): Tag = Tag(name + "#")
  val proxy = on(Tag(_))
  val proxyNamed = on(x => Tag(x))
  val method = on(top(_))
  val methodNamed = on(x => top(x))
  val module = on(Tags.of(_))
  val moduleNamed = on(x => Tags.of(x))
  val block = on(local(_))
  val blockNamed = on(x => local(x))
  val function = on(value(_))
  val functionNamed = on(x => value(x))
  val using = on(withUsing(_))
  val usingNamed = on(x => withUsing(x))
  val default = on(withDefault(_))
  val defaultNamed = on(x => withDefault(x))
  val other = on(beside(_, 1))
  val otherNamed = on(x => beside(x, 2))
  val all = List(proxy, proxyNamed, method, methodNamed, module, moduleNamed, block, blockNamed, function, functionNamed,
    using, usingNamed, default, defaultNamed, other, otherNamed)
  println(all.map(f => f("a").name).mkString(" "))
