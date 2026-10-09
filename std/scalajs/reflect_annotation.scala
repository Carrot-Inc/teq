// The annotation that marks a class for Scala.js's reflective instantiation (reflect.scala), which
// sbt-tzdb's generated `TzdbZoneRulesProvider` carries.
package scala.scalajs.reflect.annotation

import scala.annotation.StaticAnnotation

final class EnableReflectiveInstantiation extends StaticAnnotation
