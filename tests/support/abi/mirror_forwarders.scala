// A mirror class's static forwarders as dotty's `addForwarders` writes them: none for a member whose name a class of
// the object's expanded (a trait's private val or var, `mirrors$T$$pv`; `scala.App`'s `scala$App$$_args`), a
// synthetic one for a trait setter (`mirrors$T$_setter_$v_$eq`), and a plain one for a name that holds `$$` of its
// own (`a$$b`, and `` `mirrors$T$$x` `` as the source writes it). The mirror classes' alone: the objects' fields are
// scalac's static fields.
// abi: O#mirrors$T$$pv O#mirrors$T$$pv_$eq O#mirrors$T$$pvl O#mirrors$T$_setter_$mirrors$T$$pvl_$eq O#mirrors$T$_setter_$v_$eq O#v O#r X#scala$App$$_args X#scala$App$$_args_$eq X#scala$App$$initCode X#scala$App$_setter_$executionStart_$eq X#scala$App$_setter_$scala$App$$initCode_$eq X#executionStart V#a$$b W#mirrors$T$$x
package mirrors
trait T:
  private var pv = 1
  val v = 2
  private val pvl = 3
  def r = pv + pvl
object O extends T
object X extends App:
  println("x")
object V:
  def a$$b = 2
object W extends T:
  def `mirrors$T$$x`: Int = 7
