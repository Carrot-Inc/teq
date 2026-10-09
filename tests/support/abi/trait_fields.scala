// The fields a class takes from the traits it mixes in (`Mixin`'s `traitInits` and `setters`), a program trait's and
// scala-library's `App`: the trait's abstract accessors and setters, their expanded names for a private member
// (`trait_fields$T$$pv`, `scala$App$$_args`), the class's field, getter and setters, final for a final val and an
// object; a lazy val's and an object's getter over teq's own holder.
// abi: T#v T#w T#w_$eq T#lz T#trait_fields$T$$pv T#trait_fields$T$$pv_$eq T#trait_fields$T$$pvl T#prot T#fin T#constant T#Obj T#trait_fields$T$_setter_$v_$eq T#trait_fields$T$_setter_$trait_fields$T$$pvl_$eq T#trait_fields$T$_setter_$prot_$eq T#trait_fields$T$_setter_$fin_$eq Mid#v Mid#w Mid#w_$eq Mid#lz Mid#trait_fields$T$$pv Mid#trait_fields$T$$pv_$eq Mid#trait_fields$T$$pvl Mid#prot Mid#fin Mid#constant Mid#Obj Mid#trait_fields$T$_setter_$v_$eq Mid#trait_fields$T$_setter_$trait_fields$T$$pvl_$eq Mid#trait_fields$T$_setter_$prot_$eq Mid#trait_fields$T$_setter_$fin_$eq Runner#executionStart Runner#scala$App$$_args Runner#scala$App$$_args_$eq Runner#scala$App$$initCode Runner#scala$App$_setter_$executionStart_$eq Runner#scala$App$_setter_$scala$App$$initCode_$eq
package trait_fields
trait T:
  val v: Int = 1
  var w: Int = 2
  lazy val lz: Int = 3
  private var pv: Int = 4
  private val pvl: Int = 5
  protected val prot: Int = 6
  final val fin: Int = 7
  final val constant = 8
  object Obj { def x = 9 }
  def readPv: Int = pv
  def readPvl: Int = pvl
class Mid extends T
class Runner extends App
