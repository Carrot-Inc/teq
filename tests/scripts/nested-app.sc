// No main of the script's object: a nested object's `extends App` is none of its template's parents
// (Scala CLI's WrapperUtils.extendsApp), so the script's statements run, and there are none.
trait App
object Outer {
 object Inner extends App { }
}
