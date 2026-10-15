// The JDK's `java.lang.reflect.Field`, which `Class.getField` declares (scala-library's
// `Selectable.selectDynamic` asks for a field before the getter): JavaScript has no reflection, so
// the lookup fails before a `Field` is made. The JVM takes the JDK's class from `ct.sym`.
package java.lang.reflect

final class Field private () extends AccessibleObject:
  def getName: String = ""
  def get(obj: Any): AnyRef = null
