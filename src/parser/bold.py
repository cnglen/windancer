# 正确的格式能解析, 不正确的格式, org-mode为严格定义，取决于实现
# 跨行?

# pre_valid
# contents valid
# post valdi
normal_pre = set(""" \t({"'""")
normal_post = set(""" \t.,;:!?')}]\"\\\r\n""")
marker_set = set("*/_+~=")
high_priority_marker = "~="  # 不能同时存在，须在最内测
whitespace_set = set(" \t")
end_of_line_set = set("\r\n")
char2type_begin = {"*": "<bold>", "/": "<italic>", "_": "<underline>", "+": "<strikethrough>", "=": "<verbatim>", "~": "<code>"}
char2type_end = {"*": "</bold>", "/": "</italic>", "_": "</underline>", "+": "</strikethrough>", "=": "</verbatim>", "~": "</code>"}


def is_start_marker_valid(text, index: int, state) -> bool:
    """
    判断start_marker是否valid
    """
    if state and state[-1][0] in high_priority_marker:  # 上一个marker为高优先级的~=,在其内部，所有的潜在marker均不可能是marker
        return False

    # PRE:
    # - begin of line
    # - normal pre
    # - other outter marker in state
    if (index == 0) or (text[index - 1] in normal_pre) or ((text[index - 1], index - 1) in state):
        pre_valid = True
    else:
        pre_valid = False
        return False

    print("xxx", text[index], state)
    # marker: text[index] in marker_set且未在state中(限定两个相同类型的type,如bold,不能嵌套)
    if text[index] in marker_set and (text[index] not in [e[0] for e in state]):
        print("xx")
        marker_valid = True
    else:
        marker_valid = False
        return False

    # content_first_char: 不为whitespace
    print(text, index + 1, text[index + 1])
    if not (text[index + 1] in whitespace_set):
        contents_begin_valid = True
    else:
        contents_begin_valid = False

    return pre_valid and marker_valid and contents_begin_valid


# 检测marker时条件判定
# ~=: 会忽略内部的所有marker, 高优先级
# */_+：低优先级

# 最外层的严格遵守PRE/POST,内层的PRE
# 最外层: PRE POST
# 最内层: ContentBegin ContentEnd

# 成对匹配由stack保证

# 同一个标记，不能嵌套: marker start开始，检测stack(不能为stack)

# 扫描字符串，输出潜在的marker位置


def is_end_marker_valid(text, index: int, state) -> bool:
    """
    判断end_marker是否valid
    """
    # contents_last_char: 不为空
    if text[index - 1] not in whitespace_set:
        contents_end_valid = True
    else:
        contents_end_valid = False

    # marker: 和state中的last marker相同
    # 暗含state不为空
    if state and text[index] == state[-1][0]:
        marker_valid = True
    else:
        marker_valid = False

    # POST
    # - enf of str(file?)
    # - end of line
    # - 后面跟state中的某个marker, 且为history_state的逆序，且后续的字符为whitespace或eol或eof
    if index == len(text) - 1:  # end of str(file?)
        post_valid = True
    elif text[index + 1] in end_of_line_set:
        post_valid = True
    elif text[index + 1] in normal_post:
        post_valid = True
    elif text[index + 1] in [e[0] for e in state[:-1]]:
        # lookahead: POST开始的字符串，是history_state的倒序，且后续以空格/EOF/EOL结束
        tmp_state = list(e[0] for e in state[:-1])
        n_history_state = len(tmp_state)
        post_valid = False
        flag_break = False
        for j in range(index + 1, min(index + 1 + n_history_state, len(text))):
            if text[j] in tmp_state[-1]:
                tmp_state.pop()
            else:
                flag_break = True
                break

        if flag_break and text[j] in whitespace_set:
            post_valid = True
        else:
            if j + 1 < len(text):
                if text[j + 1] in whitespace_set:
                    post_valid = True
                else:
                    post_avlid = False
            else:
                post_valid = True
    else:
        post_valid = False

    return post_valid and contents_end_valid and marker_valid


def org_to_html_iterative(text: str) -> str:
    """
    预处理org-mode字符串`text`, 输出为预处理过的字符串，如有效的*a* -> <bold>a</bold?, 便于后续parser解析(后续parser不再需要判断marker是否有效)
    """
    if not text:
        return text

    i = 0
    n = len(text)

    # stack: 栈用于跟踪当前活动的标记, 每个元素是 (标记, 原始输入中的开始位置)
    stack = []
    result = []  # (i, "<bold/>") 将第i个字符替换为</bold>
    while i < n:
        # 检查是否是标记开始
        if i < n - 1 and is_start_marker_valid(text, i, stack):
            marker_char = text[i]
            stack.append((marker_char, i))  # location of result
            result.append((i, char2type_begin[marker_char]))
            i += 1

        # 检查是否是标记结束
        elif is_end_marker_valid(text, i, stack):
            marker_char, _ = stack.pop()
            result.append((i, char2type_end[marker_char]))
            i += 1
        else:
            # 普通字符
            i += 1

    result_kv = dict(result)
    # 处理未闭合的标记（视为普通字符）
    while stack:
        # 恢复result
        # print(f"stack={stack}")
        mark_char, i_input = stack.pop()
        while result and result[-1][0] >= i_input:
            (i, _) = result.pop()
            result_kv.pop(i)

    # 根据result预处理
    ans = []
    for i, c in enumerate(text):
        if i in result_kv:
            ans.append(result_kv[i])
        else:
            ans.append(text[i])
    return "".join(ans)


# 测试函数
def test_org_to_html():
    """测试函数"""
    test_cases = [
        # 基本测试
        # ("这是 *粗体* 文本", "这是 <bold>粗体</bold> 文本"),
        # ("这是 /斜体/ 文本", "这是 <italic>斜体</italic> 文本"),
        # ("这是 *粗体* 和 /斜体/ 文本", "这是 <bold>粗体</bold> 和 <italic>斜体</italic> 文本"),
        # # 边界测试
        # ("*粗体*", "<bold>粗体</bold>"),
        # ("/斜体/", "<italic>斜体</italic>"),
        # ("**不是粗体**", "<bold>*不是粗体*</bold>"),  # 连续两个*不是粗体
        # ("//不是斜体//", "<italic>/不是斜体/</italic>"),  # 连续两个/不是斜体
        # # 嵌套测试
        # ("这是 *粗体且 /斜体/* 文本", "这是 <bold>粗体且 <italic>斜体</italic></bold> 文本"),
        # ("这是 /斜体且 *粗体*/ 文本", "这是 <italic>斜体且 <bold>粗体</bold></italic> 文本"),
        # ("这是 */粗斜体/ 粗体*", "这是 <bold><italic>粗斜体</italic> 粗体</bold>"),
        # (
        #     "+/*_italic-bold-underline_*/+",
        #     "<strikethrough><italic><bold><underline>italic-bold-underline</underline></bold></italic></strikethrough>",
        # ),
        # (
        #     "+/_*italic-underline-bold*_/+",
        #     "<strikethrough><italic><underline><bold>italic-underline-bold</bold></underline></italic></strikethrough>",
        # ),
        # #
        # ("*_~inner-most~_*", "<bold><underline><code>inner-most</code></underline></bold>"),
        # ("*_~=inner-most=~_*", "<bold><underline><code>=inner-most=</code></underline></bold>"),
        # ("*_=~inner-most~=_*", "<bold><underline><verbatim>~inner-most~</verbatim></underline></bold>"),
        # ("~*_inner-most_*~", "<code>*_inner-most_*</code>"),
        # # 复杂情况
        # ("*粗体1* 普通文本 *粗体2*", "<bold>粗体1</bold> 普通文本 <bold>粗体2</bold>"),
        # ("开头 *粗体* 中间 /斜体/ 结尾", "开头 <bold>粗体</bold> 中间 <italic>斜体</italic> 结尾"),
        # ("*粗体* */正常/ 正常", "<bold>粗体</bold> */正常/ 正常"),
        # ("*/_测试_/*a", "*/_测试_/*a"),
        # ("*/_a_*/b c_*/", "*/_a_*/b c_*/"),
        # ("*/_a_*/b c_/*", "<bold><italic><underline>a_*/b c</underline></italic></bold>"),
        # # ("*/_测试_/*a bar_/*", "*/_测试_/*a"),
        # # 边缘情况
        # ("", ""),
        # ("没有标记", "没有标记"),
        # ("*", "*"),
        # ("/", "/"),
        # # aa
        # ("this is not a *bold font", "this is not a *bold font"),
        ("*_/bold-underline-italic/_*", ""),
        ("a *", "a *"),
    ]

    print("测试结果:")
    print("=" * 60)

    for i, (input_text, expected) in enumerate(test_cases, 1):
        # result_recursive = org_to_html(input_text)
        result_iterative = org_to_html_iterative(input_text)

        print(f"测试用例 {i}:")
        print(f"输入: {input_text}")
        # print(f"递归版本: {result_recursive}")
        print(f"迭代版本: {result_iterative}")
        print(f"期望: {expected}")

        # 检查是否匹配期望结果
        # recursive_match = result_recursive == expected
        iterative_match = result_iterative == expected

        # print(f"递归版本 {'✓' if recursive_match else '✗'}")
        print(f"迭代版本结果：{'✓' if iterative_match else '✗'}")
        print("-" * 40)


# 使用示例
if __name__ == "__main__":
    # # 示例文本
    # sample_text = "这是 *粗体文本* 和 /斜体文本/ ，以及 *嵌套 /斜体/ 的粗体* 。"

    # print("原始文本:", sample_text)
    # # print("递归版本:", org_to_html(sample_text))
    # print("迭代版本:", org_to_html_iterative(sample_text))

    # print("\n" + "=" * 60)
    # 运行测试
    test_org_to_html()
