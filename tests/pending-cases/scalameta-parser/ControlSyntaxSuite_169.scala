// Program 169 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 4000, "scalafmt #3790 match
// actual braces"): scalac 3.8.4's parser accepts it, teq's reports "expected 'case', found identifier". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 86
//     Defn.Object 0 85
//       Term.Name 7 11
//       Template 12 85
//         Template.Body 12 85
//           Defn.Def 14 82
//             Term.Name 18 21
//             Term.Match 26 82
//               Term.Name 26 29
//               Term.CasesBlock 36 82
//                 Case 40 81
//                   Lit.Int 45 46
//                   Term.Block 54 81
//                     Term.Apply 54 64
//                       Term.Name 54 61
//                       Term.ArgClause 61 64
//                         Lit.Int 62 63
//                     Term.Tuple 69 75
//                       Lit.Int 70 71
//                       Lit.Int 73 74
//                     Term.Name 78 81
object W169 {
def foo =
  bar match {
  case 1 =>
    println(2)
    (3, 4)
  baz}

}
