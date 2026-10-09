package plugins

import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

@EnableReflectiveInstantiation
trait Plugin:
  def name: String

class Greeter(greeting: String, tag: zeta.Tag) extends Plugin:
  def name = s"greeter $greeting ${tag.label}"

object Counter extends Plugin:
  def name = "counter"

@EnableReflectiveInstantiation
class Solo

class Kin
class Kid extends Kin
@EnableReflectiveInstantiation
class Pair(a: Kin, b: Kid)
