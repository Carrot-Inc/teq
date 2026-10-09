// jars: reflect-lib
// targets: js interp
// Classes and objects of a jar found by name at run time: the jar's TASTy carries the
// annotation, the program names none of them. The expectation is Scala.js's.
import scala.scalajs.reflect.Reflect
import reflectlib.Plugin

class Local(tag: Int) extends Plugin:
  def name = s"local $tag"

object Main:
  def main(args: Array[String]): Unit =
    for n <- List("reflectlib.Hidden", "reflectlib.Deep", "reflectlib.Middle", "reflectlib.Plugin", "reflectlib.NotPlugin", "Local",
        "reflectlib.QPriv", "reflectlib.QProt", "reflectlib.QSec", "reflectlib.Box$Item") do
      Reflect.lookupInstantiatableClass(n) match
        case None => println(s"$n: none")
        case Some(c) =>
          val ctors = c.declaredConstructors.map(_.parameterTypes.map(_.getName).mkString("(", ", ", ")"))
          println(s"$n: ${c.runtimeClass.getName} ${ctors.mkString(" ")}")
    println(Reflect.lookupInstantiatableClass("reflectlib.Hidden").get.newInstance().asInstanceOf[Plugin].name)
    println(Reflect.lookupInstantiatableClass("reflectlib.Deep").get.declaredConstructors.head.newInstance("x").asInstanceOf[Plugin].name)
    println(Reflect.lookupInstantiatableClass("reflectlib.Box$Item").get.declaredConstructors.head.newInstance(reflectlib.Box(2), 3).asInstanceOf[Plugin].name)
    println(Reflect.lookupInstantiatableClass("Local").get.declaredConstructors.head.newInstance(3).asInstanceOf[Plugin].name)
    for n <- List("reflectlib.Registry$", "reflectlib.Registry$Inner$", "reflectlib.Loose$", "reflectlib.Hidden") do
      Reflect.lookupLoadableModuleClass(n) match
        case None => println(s"$n: no module")
        case Some(m) =>
          val shown = m.loadModule() match
            case p: Plugin => p.name
            case _ => "object"
          println(s"$n: ${m.runtimeClass.getName} $shown")
