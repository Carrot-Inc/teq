// Item 4 of the JVM ABI alignment: a trait's concrete methods have
// scalac's static forwarders `m$(T, args)`, which a class scalac compiles mixing the trait in calls: an ordinary,
// a protected, a final, an `abstract override` and an extension method, a default getter, a lazy val's and a
// constant's getter, the constant's getter itself concrete in the trait (no setter); a private method, an inline
// one and an abstract accessor have none.
// abi: id id$ concrete concrete$ concrete$default$1 concrete$default$1$ prot prot$ priv$ fin fin$ inl inl$ strict strict$ laz laz$ constant constant$ variable$ ext ext$ trait_statics$T$_setter_$constant_$eq trait_statics$AO$$super$id
package trait_statics
trait Parent { def id(x: String): String; def abs: Int }
trait T extends Parent:
  def id(x: String): String = x
  def concrete(x: Int = 1): Int = x
  protected def prot(x: Long): Long = x
  private def priv(x: Int): Int = x
  final def fin: Int = priv(2)
  inline def inl: Int = 3
  val strict: Int = 4
  lazy val laz: Int = 5
  final val constant = 6
  var variable: Int = 7
  extension (x: Int) def ext(y: String): String = y
trait AO extends Parent:
  abstract override def id(x: String): String = super.id(x)
