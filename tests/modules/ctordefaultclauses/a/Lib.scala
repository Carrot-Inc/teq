package cdc

// A constructor default naming a parameter of an earlier clause: its getter takes those clauses
// (`Desugar.addDefaultGetters`' `getterParamss`), and a downstream reading the companion's
// `<init>$default$N` from the pickle reads them as the constructor's own parameters.
class Top(val x: Int = 1)(val z: Int = x * 2)
case class TopCase(x: Int = 1, y: String = "y")(val z: String = y * x)
object Holder {
  class In[A](val a: A)(val b: List[A] = List(a, a))
}
