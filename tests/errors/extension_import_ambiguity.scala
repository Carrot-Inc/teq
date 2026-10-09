// expect: 13:5: error: reference to value is ambiguous: it is both defined in class C and imported subsequently by import Ext.*
// expect: 16:3: error: reference to value is ambiguous: it is both defined in an enclosing scope and imported by name subsequently by import Ext.value
// expect: 19:3: error: reference to value is ambiguous: it is both defined in an enclosing scope and imported subsequently by import Ext.*
// expect: 3 errors found
// An imported extension method is a term like any other method, as under scalac: it makes a
// definition of an enclosing scope ambiguous, and a definition of the import's own scope wins.
object Ext:
  extension (x: Int) def value: Int = x + 1
class C:
  def value: Int = 5
  def f =
    import Ext.*
    value
def g(value: Int) =
  import Ext.value
  value
def k(value: Int) =
  import Ext.*
  value
def h =
  import Ext.*
  val value = 1
  value
