// Program 367 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 1389, "scalafmt #3720 6
// complex, with indented comments"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement,
// found identifier". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 166
//     Defn.Object 0 165
//       Term.Name 7 11
//       Template 12 165
//         Template.Body 12 165
//           Defn.Def 17 162
//             Term.Name 21 24
//             Term.ApplyInfix 31 162
//               Term.ApplyInfix 31 100
//                 Term.Apply 31 48
//                   Term.Name 31 34
//                   Term.ArgClause 34 48
//                     Term.Block 34 48
//                       Term.Name 44 48
//                 Term.Name 53 55
//                 Type.ArgClause 79 79
//                 Term.ArgClause 79 100
//                   Term.Apply 79 100
//                     Term.Name 79 82
//                     Term.ArgClause 82 100
//                       Term.Block 82 100
//                         Term.Name 96 100
//               Term.Name 109 111
//               Type.ArgClause 137 137
//               Term.ArgClause 137 162
//                 Term.Apply 137 162
//                   Term.Name 137 140
//                   Term.ArgClause 140 162
//                     Term.Block 140 162
//                       Term.Name 158 162
object W367 {

  def mtd =
    abc:
        arg1
    ++ /* c1 */ // c2
        abc:
            arg2
        ++ /*
        c3
        */ abc:
                arg3

}
