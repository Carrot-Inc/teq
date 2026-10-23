// Program 991 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 590, "type-multi-
// seq"): scalac 3.8.4's parser accepts it, teq's reports "expected an expression, found end of indented block". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 58
//     Defn.Object 0 57
//       Term.Name 7 11
//       Template 12 57
//         Template.Body 12 57
//           Defn.Val 14 54
//             Pat.Var 18 32
//               Term.Name 18 32
//             Type.Name 38 41
//             Term.Block 47 54
//               Term.Name 47 49
//               Term.Name 52 54
object W991 {
val refinementTest:
    Int = 
  fx
  fy

}
