// Program 989 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 563, "type-in-next-
// line-equals-newline"): scalac 3.8.4's parser accepts it, teq's reports "expected an expression, found end of
// indented block". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 52
//     Defn.Object 0 51
//       Term.Name 7 11
//       Template 12 51
//         Template.Body 12 51
//           Defn.Val 14 48
//             Pat.Var 18 32
//               Term.Name 18 32
//             Type.Name 40 43
//             Lit.Int 47 48
object W989 {
val refinementTest:
      Int = 
3

}
