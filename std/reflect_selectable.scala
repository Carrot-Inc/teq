// scala-library's reflective `Selectable`, which a structural call on a value of no `Selectable`
// type goes through: `x.asInstanceOf[{ def m: T }].m` under `import
// scala.reflect.Selectable.reflectiveSelectable` is `reflectiveSelectable(x).selectDynamic("m")`
// (munit's `Compat` reads a collection's `collectionClassName` so). The member is reached by its
// name on the object itself, a method called and a field read, and the call names it with a
// literal, which keeps the members of that name. JavaScript only.
package scala.reflect

trait Selectable extends scala.Selectable:
  inline def selectDynamic(inline name: String): Any = memberByName(this, name)
  inline def applyDynamic(inline name: String, inline paramTypes: Class[?]*)(inline args: Any*): Any =
    callByName(this, name, args.toArray)

object Selectable:
  implicit def reflectiveSelectable(x: Any): Selectable = unsafeCast(x)

@js("$memberByName($0, $1)")
private[reflect] def memberByName(x: Any, name: String): Any

@js("$callByName($0, $1, $2)")
private[reflect] def callByName(x: Any, name: String, args: Array[Any]): Any
