// expect: this can be used only in a class, object, or template
// expect: two or more overloaded variants of constructor D have default arguments
// expect: Traits cannot have secondary constructors
// expect: x is not accessible from constructor arguments
// expect: Conflicting definitions:
// expect: def <init>(b: Boolean): G in class G at line 24 and
// expect: secondary constructor must call a preceding constructor
// expect: None of the overloaded alternatives of constructor G in class G with types
// expect: (b: Boolean): G
// expect: (x: Int): G
// expect: match arguments (Double)
// expect: type mismatch: found Int, required String
// expect: Value classes may not define a secondary constructor
class B(x: Int):
  def this(s: String) = this(this.toString.length)
class D(x: Int = 1):
  def this(s: String, y: Int = 2) = this(s.length)
trait T:
  def this(x: Int) = this()
class E(x: Int):
  def this(s: String) = this(x)
class G(x: Int):
  def this(s: String) = this(s.length)
  def this(b: Boolean) = this(b.toString)
  def this(b: Boolean) = this(1)
class H(x: Int):
  def this(s: String) = this(s.toList)
  def this(l: List[Char]) = this(l.mkString)
class I private (x: Int):
  def this(s: String) = this(s.length)
class V(val n: Int) extends AnyVal:
  def this(s: String) = this(s.length)
object Main:
  new G(1.5)
  new I("x")
  new I(1)
