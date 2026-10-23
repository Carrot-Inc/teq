package p21
import scala.reflect.Selectable.reflectiveSelectable
object S:
  def m(x: { def foo: Int; def bar(i: Int): Int; val baz: String }) = x.foo + x.bar(1) + x.baz.length
  class Record(elems: (String, Any)*) extends Selectable:
    def selectDynamic(n: String): Any = ???
  type Rec = Record { val name: String }
  def use(r: Rec) = r.name
