// teq: --target jvm
import scala.scalajs.js

object Facades:
  def dynamic = scala.scalajs.js.Dynamic
  def field(f: java.lang.reflect.Field) = f.getName

// expect: value scalajs is not a member of scala
