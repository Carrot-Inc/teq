package app

import scala.scalajs.reflect.Reflect
import plugins.Plugin

// The plugins are registered by their module, which nothing here imports.
@main def run(): Unit =
  val greeter = Reflect.lookupInstantiatableClass("plugins.Greeter").get
  val ctor = greeter.declaredConstructors.head
  println(ctor.parameterTypes.map(_.getName).mkString(", "))
  println(ctor.newInstance("hi", zeta.Tag("t")).asInstanceOf[Plugin].name)
  println(Reflect.lookupLoadableModuleClass("plugins.Counter$").get.loadModule().asInstanceOf[Plugin].name)
  println(Reflect.lookupInstantiatableClass("plugins.Plugin"))
  println(Reflect.lookupInstantiatableClass("plugins.Solo").isDefined)
