// A String's own members in a selection are the JDK's (java.lang.String's contains(CharSequence)), not the std's
// implementation's wider contains(String | Char): a Char fits only C's contains, which scalac selects.
trait T; trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def contains(c: Char): Boolean = false }
object O:
  def f(using T): C = new C
  def f(using U): String = "abc"
@main def run(): Unit = println(O.f.contains('b'))
