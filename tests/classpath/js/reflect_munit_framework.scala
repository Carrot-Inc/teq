// jars: munit munit-diff
// targets: js interp
//> using platform js
//> using dep org.scalameta::munit::1.3.4
// A framework of a jar found by name at run time by a program that names nothing of sbt's test
// interface: munit's class inherits `@EnableReflectiveInstantiation` from sbt's `Framework`, a
// trait of the std, whose file enters for the search alone. The expectation is Scala.js 1.22's
// (`scala-cli run --js-version 1.22.0`).
import scala.scalajs.reflect.Reflect

@main def run(): Unit =
  val found = Reflect.lookupInstantiatableClass("munit.Framework")
  println(found.map(_.runtimeClass.getName))
  println(found.toList.flatMap(_.declaredConstructors.map(_.parameterTypes.length)))
  println(found.get.newInstance().getClass.getName)
