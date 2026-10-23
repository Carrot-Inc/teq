// Program 411 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 2327, "#4152 not fewer braces:
// type"): scalac 3.8.4's parser accepts it, teq's reports "expected ',' or ']', found ':'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 45
//     Defn.Object 0 44
//       Term.Name 7 11
//       Template 12 44
//         Template.Body 12 44
//           Defn.Def 14 41
//             Term.Name 18 19
//             Member.ParamClauseGroup 19 35
//               Type.ParamClause 19 35
//                 Type.Param 20 34
//                   Type.Name 20 21
//                   Type.ParamClause 21 21
//                   Type.Bounds 21 34
//                     Type.Name 31 34
//             Term.Name 38 41
object W411 {
def A[B:
        Seq] = ???

}
