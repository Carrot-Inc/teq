// Refinements with term members are structural: reached through `selectDynamic` and
// `applyDynamic` on a `Selectable`, with the types the refinement declares.
class Record(fields: Map[String, Any], methods: Map[String, Int => Any]) extends Selectable:
  def selectDynamic(name: String): Any = fields(name)
  def applyDynamic(name: String)(args: Any*): Any = methods(name)(args.head.asInstanceOf[Int])

type Person = Record { def name: String; def age: Int; def older(by: Int): Int; val tag: Long }

def person(name: String, age: Int): Person =
  Record(Map("name" -> name, "age" -> age, "tag" -> 7L), Map("older" -> ((by: Int) => age + by))).asInstanceOf[Person]

trait Shape:
  type Measure
  def size: Measure

@main def main(): Unit =
  val p = person("Ada", 36)
  println(p.name)
  println(p.age + 1)
  println(p.older(4))
  println(p.tag)
  val counted: Shape { type Measure = Int } = new Shape:
    type Measure = Int
    def size: Measure = 3
  val n: Int = counted.size
  println(n * 2)
  val labelled: AnyRef { def label: String } = new AnyRef { def label = "anon" }
  println(labelled.toString.length > 0)
