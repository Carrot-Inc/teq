// scala-library's reflective `Selectable`, which a structural call on a value of no `Selectable`
// type goes through: `x.asInstanceOf[{ def m: T }].m` under `import
// scala.reflect.Selectable.reflectiveSelectable` is `reflectiveSelectable(x).selectDynamic("m")`
// (munit's `Compat` reads a collection's `collectionClassName` so). The member is reached by its
// name on the object itself, a method called and a field read, and the call names it with a
// literal, which keeps the members of that name; a method by its name and the classes of its
// parameters. JavaScript and the interpreter.
package scala.reflect

trait Selectable extends scala.Selectable:
  inline def selectDynamic(inline name: String): Any = memberByName(this, name)
  inline def applyDynamic(inline name: String, inline paramTypes: Class[?]*)(inline args: Any*): Any =
    callBySignature(this, name, paramTypes, args.toArray)

object Selectable:
  implicit def reflectiveSelectable(x: Any): Selectable = unsafeCast(x)

@js("$memberByName($0, $1)")
private[reflect] def memberByName(x: Any, name: String): Any

// The method of the name whose parameters erase to the classes: an alternative of an overloaded
// name goes by its name and those classes in the output, the call naming it by the classes it is
// given as literals (`typer::reflective_names`), as Scala.js's reflective proxies are named.
@js("$callBySignature($0, $1, $2, $3)")
private[reflect] def callBySignature(x: Any, name: String, paramTypes: Seq[Class[?]], args: Array[Any]): Any
