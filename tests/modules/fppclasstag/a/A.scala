package classtag
import scala.reflect.ClassTag
class Base(using val ct: ClassTag[List[Int]])
trait T extends Base
class K extends T
