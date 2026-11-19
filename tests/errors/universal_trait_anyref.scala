// A universal trait derives from no `Object`, so it is no `AnyRef` (dotty's `Namer.ensureFirstIsClass` puts
// `Object` before a trait's first parent unless it is `Any`; `ClassfileParser.parseParents` reads Java's
// `Serializable` and `Cloneable` so): scala-library's `Product`, `Equals` and `reflect.Enum`, a trait of the program
// whose first parent resolves to `Any`, written so or through an alias, their intersection and a type parameter
// bounded by one; master took them all for `AnyRef`. A trait extending one without `Any`, a class, `null` and a Java
// interface stay as they were.
// expect: 22:29: error: Found:    Product⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 23:42: error: Found:    Serializable⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 24:40: error: Found:    Enum⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 25:28: error: Found:    Equals⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 26:41: error: Found:    Cloneable⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 27:23: error: Found:    U⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 28:23: error: Found:    R⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 29:44: error: Found:    Product & Serializable⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// expect: 30:37: error: Found:    T⏎Required: AnyRef⏎Note that implicit conversions were not tried because the result of an implicit conversion⏎must be more specific than AnyRef
// absent: universal_trait_anyref.scala:31:
// absent: universal_trait_anyref.scala:32:
// absent: universal_trait_anyref.scala:33:
// absent: universal_trait_anyref.scala:34:
// absent: universal_trait_anyref.scala:35:
// absent: universal_trait_anyref.scala:36:
def f(x: Product): AnyRef = x
def g(x: java.io.Serializable): AnyRef = x
def h(x: scala.reflect.Enum): AnyRef = x
def k(x: Equals): AnyRef = x
def c(x: java.lang.Cloneable): AnyRef = x
def u(x: U): AnyRef = x
def a(x: R): AnyRef = x
def i(x: Product & Serializable): AnyRef = x
def t[T <: Product](x: T): AnyRef = x
def w(x: W): AnyRef = x
def v(x: V): AnyRef = x
def n(x: java.lang.Runnable): AnyRef = x
def s(x: CharSequence): AnyRef = x
def o(x: Some[Int]): AnyRef = x
def z: Product = null
trait U extends Any
type Root = Any
trait R extends Root
trait V extends U
trait W extends Product
