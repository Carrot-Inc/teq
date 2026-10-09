// The operations of js.UndefOr. The alias A | Unit carries no companion, so they live in package
// scala, which every file sees.
package scala

import scala.scalajs.js

extension [A](x: A | Unit)
  def toOption: Option[A] = if js.isUndefined(x) then None else Some(x.asInstanceOf[A])
  def getOrElse[B >: A](default: => B): B = if js.isUndefined(x) then default else x.asInstanceOf[A]
  def isDefined: Boolean = !js.isUndefined(x)
  def map[B](f: A => B): B | Unit = if js.isUndefined(x) then () else f(x.asInstanceOf[A])
