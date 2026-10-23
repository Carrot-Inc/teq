// Program 990 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 573, "type-equals-
// separate"): scalac 3.8.4's parser accepts it, teq's reports "unmatched closing bracket". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 60
//     Defn.Object 0 59
//       Term.Name 7 11
//       Template 12 59
//         Template.Body 12 59
//           Defn.Def 14 56
//             Term.Name 18 32
//             Member.ParamClauseGroup 32 52
//               Type.ParamClause 32 32
//               Term.ParamClause 32 52
//                 Term.Param 33 51
//                   Term.Name 33 34
//                   Type.Name 43 46
//                   Lit.Int 50 51
//             Term.Name 55 56
object W990 {
def refinementTest(a :
      Int = 
3) = a

}
