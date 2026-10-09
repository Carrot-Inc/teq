"""Compares teq's reconstruction of a pickle's positions (`teq tasty --positions`) with what scalac
3.8.4's unpickler gives the trees (the TastyInspector program Oracle.scala beside this file): per
kind of tree, the set of spans (start, end, point) each side has. A kind is a scalac tree class;
teq's tags map onto them as TreeUnpickler reads them, a shared reference as its target, the
leaves (identifiers, selections, paths, literals, type trees) one group. Prints the differences
and exits 1 when there is one.

usage: compare.py <teq listing> <oracle listing>; check.py runs it over the fixtures."""
import sys
from collections import defaultdict

TAGS = {
    'APPLY': 'Apply', 'APPLYsigpoly': 'Apply', 'THROW': 'Apply', 'TYPEAPPLY': 'TypeApply', 'TYPED': 'Typed',
    'ASSIGN': 'Assign', 'BLOCK': 'Block', 'INLINED': 'Inlined', 'IF': 'If', 'LAMBDA': 'Closure', 'MATCH': 'Match',
    'TRY': 'Try', 'RETURN': 'Return', 'WHILE': 'WhileDo', 'REPEATED': 'SeqLiteral', 'BIND': 'Bind',
    'ALTERNATIVE': 'Alternative', 'UNAPPLY': 'UnApply', 'CASEDEF': 'CaseDef', 'VALDEF': 'ValDef', 'PARAM': 'ValDef', 'SELFDEF': 'ValDef',
    'DEFDEF': 'DefDef', 'TYPEDEF': 'TypeDef', 'TYPEPARAM': 'TypeDef', 'TEMPLATE': 'Template', 'NEW': 'New',
    'NAMEDARG': 'NamedArg', 'SUPER': 'Super', 'APPLIEDtpt': 'AppliedTypeTree', 'TYPEBOUNDStpt': 'TypeBoundsTree',
    'LAMBDAtpt': 'LambdaTypeTree', 'REFINEDtpt': 'RefinedTypeTree', 'ANNOTATEDtpt': 'Annotated',
    'BYNAMEtpt': 'ByNameTypeTree', 'SINGLETONtpt': 'SingletonTypeTree', 'MATCHtpt': 'MatchTypeTree',
    'PACKAGE': 'PackageDef', 'IMPORT': 'Import', 'EXPORT': 'Export', 'QUOTE': 'Quote', 'SPLICE': 'Splice',
    'QUOTEPATTERN': 'QuotePattern', 'SPLICEPATTERN': 'SplicePattern', 'SELECTouter': 'Select',
}
SCALAC = {'InlineIf': 'If', 'InlineMatch': 'Match', 'SubMatch': 'Match', 'JavaSeqLiteral': 'SeqLiteral'}
LEAVES = {'Ident', 'Select', 'This', 'Literal', 'TypeTree', 'InferredTypeTree'}


def kind_of_tag(tag):
    if '(' in tag:
        tag = tag[tag.index('(') + 1:-1]
    k = TAGS.get(tag)
    return k if k else 'leaf'


def span_of(words):
    if not words or words[0] == 'none' or not words[0].isdigit() or len(words) < 3 or not words[2].isdigit():
        return None
    start, end = int(words[0]), int(words[2])
    point = int(words[4]) if len(words) > 4 and words[3] == 'point' else None
    return (start, end, point)


def teq(lines):
    out = defaultdict(set)
    for line in lines:
        if line.startswith('//') or not line.strip():
            continue
        w = line.split()
        span = span_of(w[2:])
        if span is not None:
            out[kind_of_tag(w[1])].add(span + (w[-1],))
    return out


def oracle(lines):
    out = defaultdict(set)
    for line in lines:
        if line.startswith('#') or not line.strip():
            continue
        w = line.split()
        span = span_of(w[1:])
        if span is None:
            continue
        k = SCALAC.get(w[0], w[0])
        out['leaf' if k in LEAVES else k].add(span + (w[-1],))
    return out


def differences(t, o):
    """The lines saying what differs between teq's spans `t` and scalac's `o`, by kind."""
    out = []
    for k in sorted(set(t) | set(o)):
        if k == 'leaf':
            # scalac makes leaves teq's walk does not list (a path's prefix, a `throw`), and
            # teq lists leaves scalac drops into a parent (a constructor's `()`): the leaves are
            # compared one way, what teq gives against what scalac has.
            missing = t[k] - o[k]
            extra = set()
        else:
            missing, extra = t[k] - o[k], o[k] - t[k]
        out.extend(f'{k}: teq {s}, not scalac\'s' for s in sorted(missing, key=str))
        out.extend(f'{k}: scalac {s}, not teq\'s' for s in sorted(extra, key=str))
    return out


def lines_teq(text):
    return text.splitlines()


if __name__ == '__main__':
    t, o = teq(open(sys.argv[1])), oracle(open(sys.argv[2]))
    diff = differences(t, o)
    print('\n'.join(diff))
    print(f'{sum(len(v) for v in t.values())} spans of teq, {sum(len(v) for v in o.values())} of scalac, {len(diff)} differences')
    sys.exit(1 if diff else 0)
