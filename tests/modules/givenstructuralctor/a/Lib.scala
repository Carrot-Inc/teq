package gsca

// Structural givens over constructor calls (dotty's `Parsers.givenDef`): a given object whose
// class extends the call and holds its body's members, and a given class with a using clause,
// read over the products.
trait Parent(val n: Int):
  def tag: String
trait Show[A] { def s: String }
object O:
  given t: Parent(1):
    def tag = "t" + n
    def extra = "ok"
  given showAll[A](using k: Int): Show[A] with
    def s = "s" + k
// A parameterized one answers its class, whose added members the downstream reads over the
// products.
object P:
  given withCount: (k: Int) => Parent(k):
    def tag = "p" + k
    def doubled = k * 2
