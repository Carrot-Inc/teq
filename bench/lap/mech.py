# mech.py <name> <program>...: cachegrind's counts (cgr.sh) by mechanism, $LX/teq-<name> against
# master, in percent of master's count: the functions grouped by the mechanism their name belongs
# to, a master function the landing split into a locked wrapper and its `_unlocked` body counted
# under the body's name; what the mechanisms' code executes inlined into other functions is
# counted with those, under "elsewhere".
import os, sys, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from fdiff import load
LX = os.environ.get('LX', '/tmp/pt3-lap/lx')
GROUPS = [
 ('shared stores', r'teq::shared::(Table|TableBuf|SlabVec|Serial)|teq::intern::Interner|TypeStore>::(mk|list|lit|refine|blocked|match_type|term_unshared|reserve|release_retired)|teq::types::Type as core::(hash|cmp)|hash_one::<&teq::types::Type>|equivalent_key::<teq::types::Type|RawTable<\((teq::types::Type, teq::types::TypeId|alloc::boxed::Box<\[teq::types::TypeId\]>, teq::types::TList|&str, teq::intern::Name)\)>|HashMap<&str, teq::intern::Name'),
 ('arenas', r'teq::arena::(Arena|SharedArena|Parallel|Dense|FileVec|FileMaps|Layered|SharedMap|Bits|Registers|place)|teq::tir::Program>::(add|list|syms|expr|pat|str)|teq::symbols::Symbols>::(new_sym|new_tparam|new_class|new_alias|class|sym|tparam|alias|pkg|add_member)\b'),
 ('cells', r'teq::arena::(Cells|CellRef|CellDirectory)|teq::shared::(CellState|notify|flush_staged|wait_until)|Worker>::(sig_of|sig_arc|completed_sig|uncompleted_sig|complete_sig_inner|publish_sig|settled|settled_unlocked|complete_class|complete_class_inner|complete_class_uncompleted|complete_alias|wait_cell|check_class_for_run)$'),
 ('queue, merge, loader lock', r'Worker>::(walk_body|import_count|check_item|check_top_def|type_member_body_with|with_loader.*|lock_taken|lock_released|ensure_body|check_file)$|_unlocked$|teq::typer::merge|Walk>'),
]
def pair(m, l):
    # A master function the landing split into a locked wrapper and an `_unlocked` body is
    # counted under the body's name on both sides, so that the move is not a group's cost.
    out = dict(m)
    for k in l:
        if k.endswith('_unlocked') and k[:-len('_unlocked')] in out:
            out[k] = out.pop(k[:-len('_unlocked')])
    return out
for p in sys.argv[2:]:
    m, tm = load(f'{LX}/cgr-master-{p}.fn'); l, tl = load(f'{LX}/cgr-{sys.argv[1]}-{p}.fn')
    m = pair(m, l)
    row = []; rest_m = dict(m); rest_l = dict(l)
    for name, pat in GROUPS:
        ms = ls = 0
        for k in list(rest_m):
            if re.search(pat, k): ms += rest_m.pop(k)
        for k in list(rest_l):
            if re.search(pat, k): ls += rest_l.pop(k)
        row.append((ls - ms) / tm * 100)
    row.append((sum(rest_l.values()) - sum(rest_m.values())) / tm * 100)
    row.append((tl - tm) / tm * 100)
    print(f'| {p} | ' + ' | '.join(f'{x:+.2f}' for x in row) + ' |')
