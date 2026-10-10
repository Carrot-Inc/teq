// scala-library's reflective `Selectable`, which a structural call on a value of no `Selectable`
// type goes through: `x.asInstanceOf[{ def m: T }].m` under `import
// scala.reflect.Selectable.reflectiveSelectable` is `reflectiveSelectable(x).selectDynamic("m")`
// (munit's `Compat` reads a collection's `collectionClassName` so). A member is selected as
// scala-library's `selectDynamic` finds a Scala class's, by the method of its name and no
// parameters (`applyDynamic(name)()`), and called by its name and the classes of its parameters'
// erasures as `getMethod(name, paramTypes*)` finds it, a missing one a `NoSuchMethodException`
// of `Class.methodToString`'s message. JavaScript and the interpreter.
package scala.reflect

trait Selectable extends scala.Selectable:
  inline def selectDynamic(inline name: String): Any = applyDynamic(name)()
  inline def applyDynamic(inline name: String, inline paramTypes: Class[?]*)(inline args: Any*): Any =
    callBySignature(this, name, paramTypes, args.toArray, noSuchMethod)

object Selectable:
  implicit def reflectiveSelectable(x: Any): Selectable = unsafeCast(x)

// The method of the name whose parameters erase to the classes, by its reflective proxy (Scala.js's
// `MethodName.reflectiveProxy`), named by the classes the call gives as literals: the classes of
// the erasures, a value class's its own, an array's of its element and dimensions
// (`typer::reflective_proxy_name`).
@js("$callBySignature($0, $1, $2, $3, $4)")
private[reflect] def callBySignature(x: Any, name: String, paramTypes: Seq[Class[?]], args: Array[Any], fail: (Any, String) => Nothing): Any

private[reflect] def noSuchMethod(x: Any, signature: String): Nothing =
  throw new NoSuchMethodException(x.getClass.getName + "." + signature)
