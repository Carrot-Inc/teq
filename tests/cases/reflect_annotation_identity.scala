//> using platform js
//> using jsVersion 1.21.0
// interp-expected: js
// The annotation is Scala.js's class, whatever it is called where it is written: under an alias
// it counts, and a class of the program that takes its name does not.
package ident

import scala.scalajs.reflect.Reflect
import scala.scalajs.reflect.annotation.{EnableReflectiveInstantiation as Reflective}

class EnableReflectiveInstantiation extends scala.annotation.StaticAnnotation

@EnableReflectiveInstantiation
class Mine

@Reflective
class Aliased

@scala.scalajs.reflect.annotation.EnableReflectiveInstantiation
class Spelled

@EnableReflectiveInstantiation
trait Marked
class BelowMine extends Marked

@main def main(): Unit =
  for n <- List("ident.Mine", "ident.Aliased", "ident.Spelled", "ident.BelowMine") do
    println(s"$n ${Reflect.lookupInstantiatableClass(n).isDefined}")
