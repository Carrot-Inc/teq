// Program 514 of tests/fix/scalameta, from scalameta 4.17.4's GivenUsingSuite (line 56, "given-multiple-
// inheritance"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found identifier". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 84
//     Defn.Object 0 83
//       Term.Name 7 11
//       Template 12 83
//         Template.Body 12 83
//           Defn.Def 16 81
//             Term.Name 20 21
//             Term.Block 24 81
//               Defn.Given 26 79
//                 Term.Name 32 36
//                 Template 38 79
//                   Init 38 46
//                     Type.Apply 38 46
//                       Type.Name 38 41
//                       Type.ArgClause 41 46
//                         Type.Name 42 45
//                     Name.Anonymous 47 47
//                   Init 52 59
//                     Type.Apply 52 59
//                       Type.Name 52 54
//                       Type.ArgClause 54 59
//                         Type.Name 55 58
//                     Name.Anonymous 60 60
//                   Template.Body 65 79
//                     Defn.Def 67 77
//                       Term.Name 71 73
//                       Lit.Int 76 77
object W514 {
  def w = {
given intM: Ord[Int] with Eq[Int] with { def fx = 3 }
}
}
