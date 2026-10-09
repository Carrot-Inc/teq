//> using options -language:strictEquality
// teq: --strict-equality
// The library is compiled apart: strict equality checks the program's comparisons, not the
// ones in the bodies of the library classes the program reaches.
import java.util.HashMap

enum Color derives CanEqual:
  case Red, Green

@main def run(): Unit =
  val counts = new HashMap[String, Integer]()
  val first = counts.computeIfAbsent("a", _ => Integer.valueOf(1))
  val again = counts.computeIfAbsent("a", _ => Integer.valueOf(2))
  println(s"$first $again ${counts.get("a")}")
  // Both `canEqualOption` and `canEqualOptions` fit these: the first one found is taken.
  val color: Option[Color] = Some(Color.Red)
  println(s"${color == Some(Color.Red)} ${color == None} ${List(Color.Red) == List(Color.Green)}")
