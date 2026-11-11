// An enum whose constructor parameter has a default, a case leaving it out (`E1`) and one giving it (`E2`). teq's JVM,
// JavaScript and interpreter runs print the .expected, scalac 3.8.4's; its products build (`compiler build --target
// jvm --products`) fails in the TASTy writer: "E: references to definitions never written: E2 (..), E1 (..)"
// (src/tasty/write/pickle.rs, the pickle of E references the cases' definitions and writes none). Cause not located.
// Found by reviewer #13 of the workers exports helper (push 8); the same on integration b602956824.
enum E(val w: Int = 11):
  case E1 extends E()
  case E2 extends E(5)

@main def run(): Unit =
  println(E.E1.w)
  println(E.E2.w)
  println(E.values.map(_.w).mkString(","))
