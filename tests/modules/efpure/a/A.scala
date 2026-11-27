// The readers' contract of a local class's default getter: ONE source, the getter the class declares (a member
// DefaultGetter-named, as the class's pickle holds it), registered as its parameter's getter; the parameter's
// default a view of that member, its binders the method's by clause and its own result type kept. Every reader
// takes that member:
//   the typer's getter table (GetterTable)                 the declared member as the getter of (the method, the index)
//   the loader's body conversion                           the member, known by its name kind, never by its spelling
//   the JVM's omitted argument (method_default_route)      a call of the member by dispatch, the lowered DefaultRoute too
//   the JVM's class writer (TFun::own_defaults)            no second getter beside the one the class declares
//   JavaScript's omitted argument (emit_default)           `recv.<getter>(..)`, the member by dispatch, overloads too
//   JavaScript's class writer (emit_method_getters)        the member, by identity and not by its emitted name
//   the interpreter (default_target, a getter's own call)  one definition, entered by the default and by the call
//   quoted reflection (default_getter_name)                the member as the getter of its parameter
//   the products' pickle writer                            one getter, its name kind, signature and binders kept
//   the language server's completion (describe_candidate)  a parameter's default where the getter is
//   wildcard exports (forwardable_names)                   the member, known as a getter by its name kind
//   the used names (deps.rs default_getter)                the getter's name, the member's
// The tests/modules/ef* cases hold a module's local class on these readers, over teq's products and over scalac's
// classes (over-scalac.txt, over-scalac-js.txt); the legs master fails are in tests/modules/known.txt, each with its
// mechanism and the family it belongs to.
package esc
trait API { def m(a: Int)(b: Int = a + 7): Int }
object Lib {
  def make(x: Int): API = {
    trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = 10 + a + b }
    class C extends T { override def m(a: Int)(b: Int): Int = super.m(a)(b) }
    new C
  }
}
