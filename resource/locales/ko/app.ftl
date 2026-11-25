# SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
# Games Fair: Korean UI strings. The keys, their order and the
# comments follow locales/en/app.ftl, which is the catalog they are generated against.

# The name comes from app metadata; translations can place it without duplicating the brand.
app_title = { $title }

nav_breakout = 벽돌깨기
nav_sirtet = 떨어지는 블록
nav_sudoku = 스도쿠
nav_2048 = 2048
nav_blockblast = 블록 폭파
nav_solitaire = 솔리테어

gk_close = 게임 닫기

# Shared game chrome (gamekit::chrome)
gk_pause = 일시정지
gk_paused = 일시정지
gk_resume = 계속하기
gk_new_game = 새 게임
gk_settings = 설정
gk_instructions = 방법
gk_play_again = 다시 하기
gk_game_over = 게임 오버
gk_score = 점수
gk_best = 최고 기록
gk_new_high_score = 최고 점수 갱신!
gk_sounds = 효과음
gk_vibrations = 진동
gk_data = 데이터
gk_done = 완료
gk_cancel = 취소
gk_reset_high_score = 최고 점수 초기화
gk_reset_high_score_title = 최고 점수를 초기화할까요?
gk_reset_high_score_message = 최고 점수가 영구적으로 0으로 초기화됩니다.
gk_reset_confirm = 초기화
gk_game_over_heading = 게임 종료

# The app's Settings (src/lib.rs)
settings_sound = 소리 및 진동
settings_sound_footer = 각 게임의 효과음과 진동 스위치는 처음 열 때 이 설정으로 시작합니다. 특정 게임만 바꾸려면 그 게임의 일시정지 메뉴에서 조정하세요.
settings_records = 기록
settings_reset_records = 모든 기록 초기화
settings_reset_records_caption = 모든 게임의 최고 점수, 최고 기록 시간, 승리 횟수.
settings_reset_records_confirm = 모든 게임의 최고 점수, 최고 기록 시간, 승리 횟수가 0으로 돌아갑니다. 기록은 이 기기에만 저장되며 복구할 수 없습니다.

# Breakout
bk_level_caption = 레벨
bk_lives = 목숨
bk_tap_to_launch = 탭하여 발사
bk_drag_to_move = 드래그하여 패들 이동
bk_level_clear = 레벨 { $n } 완료!
bk_next_level = 다음 레벨
bk_help_intro = 패들로 공을 튕겨 모든 벽돌을 깨세요.
bk_help_play = 게임 방법
bk_help_play_1 = • 게임 영역을 **드래그**하면 패들이 움직입니다. 손가락 위에 표시되며, 빠르게 위로 밀면 공을 더 세게 칩니다.
bk_help_play_2 = • 공을 놓치지 마세요. 패들 아래로 떨어지면 목숨 하나를 잃습니다.
bk_help_play_3 = • 모든 벽돌을 깨면 다음 레벨로 갑니다. 빠르게 연속으로 깨면 점수 배율이 올라갑니다.
bk_help_powerups = 강화 아이템
bk_help_powerups_1 = 일부 벽돌에서 아이템이 떨어집니다. 패들로 받으면 잠시 넓어지는 패들, 느린 공, 관통하는 공, 추가 공이나 목숨을 얻습니다.
bk_help_lives = 목숨과 게임 종료
bk_help_lives_1 = 목숨은 3개로 시작하며 모두 잃으면 끝납니다. 레벨을 완료할 때마다 속도가 빨라지고 여러 번 맞혀야 깨지는 벽돌도 나옵니다.

# Sirtet
st_level = 레벨
st_lines = 줄
st_next = 다음
st_next_a11y = 다음 블록
st_game_over = 게임 오버
st_clear_single = 한 줄
st_clear_double = 두 줄
st_clear_triple = 세 줄
st_clear_sirtet = 네 줄!
st_help_intro = 떨어지는 블록을 쌓아 가로줄을 채우세요. 화면이 가득 차면 안 됩니다.
st_help_play = 게임 방법
st_help_play_1 = • **좌우로 드래그**하거나 방향키로 블록을 움직이세요.
st_help_play_2 = • **탭**하여 블록을 회전하세요.
st_help_play_3 = • **아래로 드래그**하면 더 빨리 떨어집니다.
st_help_lines = 줄 없애기
st_help_lines_1 = • 가로 한 줄을 모두 채우면 사라집니다.
st_help_lines_2 = • **여러 줄을 한꺼번에** 없애면 추가 점수를 얻습니다. 네 줄이 가장 큰 보상입니다.
st_help_levels = 레벨과 속도
st_help_levels_1 = 줄을 없앨수록 레벨이 올라가고 블록이 빨라집니다.
st_help_over_1 = 새 블록이 들어올 수 없으면 게임이 끝납니다.

# 2048
tf_game_over = 게임 오버
tf_keep_going = 계속하기
tf_won_title = 2048!
tf_difficulty = 난이도
tf_easy = 쉬움
tf_normal = 보통
tf_hard = 어려움
tf_detail_easy = 2만 등장. 한 게임에 되돌리기 3회.
tf_detail_normal = 기본 규칙: 2가 90%, 4가 10%.
tf_detail_hard = 4가 20%. 매번 타일 두 개 등장.
tf_choose_difficulty = 난이도 선택
tf_undo = 되돌리기 ({ $n })
tf_undo_a11y = 마지막 이동 되돌리기
tf_won_message = 2048 타일을 만들었어요! 더 높은 점수에 도전할까요?
tf_help_intro = 4×4 보드에서 같은 숫자를 합쳐 **2048**을 만드세요.
tf_help_play = 게임 방법
tf_help_play_1 = • 상하좌우로 **밀면** 모든 타일이 이동합니다. 방향키도 사용할 수 있습니다.
tf_help_play_2 = • **같은 숫자**끼리 만나면 두 숫자의 **합**으로 바뀝니다.
tf_help_play_3 = • 이동할 때마다 빈칸에 **2**, 때로는 **4**가 무작위로 생깁니다.
tf_help_goal = 목표
tf_help_goal_1 = {"*"}*2048**을 만들면 승리합니다. 이후에도 계속 점수를 올릴 수 있습니다.
tf_help_over_1 = 보드가 가득 차고 이웃한 같은 숫자가 없으면 끝납니다.
tf_help_tips = 도움말
tf_help_tips_1 = • 가장 큰 타일을 모서리에 두세요.
tf_help_tips_2 = • 한 방향으로 합쳐 작은 타일을 큰 타일 아래에 정렬하세요.
tf_help_tips_3 = • 쉬움에서는 **되돌리기**로 실수를 고칠 수 있습니다.

# Block Blast
bb_board_a11y = 블록 폭파 보드와 대기 블록
bb_no_moves = 놓을 수 있는 블록이 없습니다.
bb_combo = 콤보 ×{ $n }
bb_lines = { $n }줄
bb_msg_nice = 굿!
bb_msg_good = 좋아!
bb_msg_sweet = 달콤해!
bb_msg_great = 훌륭해!
bb_msg_smooth = 부드러워!
bb_msg_slick = 매끄러워!
bb_msg_awesome = 대단해!
bb_msg_excellent = 훌륭해!
bb_msg_fantastic = 환상적!
bb_msg_amazing = 놀라워!
bb_msg_incredible = 놀라워!
bb_msg_spectacular = 장관!
bb_msg_unbelievable = 믿을 수 없어!
bb_msg_legendary = 전설!
bb_msg_unstoppable = 막을 수 없어!
bb_msg_perfect = 퍼펙트 클리어!
bb_msg_flawless = 완벽해!
bb_easy = 쉬움
bb_normal = 보통
bb_hard = 어려움
bb_detail_easy = 세 블록을 모두 놓을 수 있습니다.
bb_detail_normal = 대부분 놓을 수 있는 균형 잡힌 난이도.
bb_detail_hard = 무작위 블록. 공간이 부족할 수 있습니다.
bb_choose_difficulty = 난이도 선택
bb_help_intro = 8×8 보드에 블록을 놓고 **가로줄**이나 **세로줄**을 채워 없애세요.
bb_help_play = 게임 방법
bb_help_play_1 = • 대기 블록을 보드로 **드래그**하세요. 손가락 위에 뜨며 그림자는 놓일 위치를, 빛은 완성될 줄을 보여 줍니다.
bb_help_play_2 = • 블록은 회전하지 않습니다. 세 개를 모두 놓으면 새 블록 세 개가 나옵니다.
bb_help_play_3 = • **1**, **2**, **3**으로 선택하고 방향키로 이동한 뒤 같은 숫자를 다시 눌러 놓으세요.
bb_help_score = 점수 계산
bb_help_score_1 = • 작은 칸 하나당 10점입니다. 없앤 줄 수의 제곱 × 100점을 얻으므로 네 줄은 1,600점입니다.
bb_help_score_2 = • 연속으로 줄을 없애면 **콤보**가 최대 ×4까지 오릅니다. 두 번 연속 줄을 못 없애도 유지되지만 세 번째에는 끊깁니다.
bb_help_score_3 = • 보드를 비우면 **5,000점** 보너스!
bb_help_difficulty = 난이도
bb_help_difficulty_1 = • **쉬움**은 항상 배치 가능, **보통**은 대체로 가능, **어려움**은 무작위입니다.
bb_help_over_1 = 어떤 블록도 놓을 수 없으면 끝납니다. 배치할 수 없는 블록은 어둡게 표시됩니다.

# Solitaire
sol_table_a11y = 솔리테어 카드판
sol_time = 시간
sol_moves = 이동 수
sol_hint = 힌트
sol_undo = 실행 취소
sol_draw_one = 한 장 뽑기
sol_draw_three = 세 장 뽑기
sol_detail_one = 덱에서 한 장씩 뒤집습니다.
sol_detail_three = 세 장씩 뒤집으며 맨 위 카드만 사용합니다.
sol_choose = 게임 선택
sol_shuffling = 섞는 중…
sol_finding = 이길 수 있는 카드 배치를 찾는 중
sol_suit_done = 무늬 완성!
sol_you_win = 승리!
sol_new_best_time = 최고 기록 갱신!
sol_best_time = 최단 시간
sol_record = { $played }판 중 { $won }승 · { $streak }연승
sol_stuck_title = 더는 움직일 수 없음
sol_stuck_message = 낼 수 있는 카드가 없습니다. 몇 수 되돌리거나 다시 나누세요.
sol_no_line = 현재 상태에서는 이길 수 없음
sol_try_undo = 몇 수 되돌리고 다른 방법을 시도하세요.
sol_winnable = 이길 수 있는 배치만
sol_winnable_detail = 풀이기가 먼저 승리할 수 있는지 확인합니다.
sol_statistics = 통계
sol_stats_one = 한 장 뽑기: { $played }판 중 { $won }승
sol_stats_three = 세 장 뽑기: { $played }판 중 { $won }승
sol_reset_stats = 통계 초기화
sol_reset_stats_title = 통계를 초기화할까요?
sol_reset_stats_message = 솔리테어의 승리, 연승, 최단 시간 기록을 영구적으로 삭제합니다.
sol_help_intro = 같은 무늬끼리 **에이스부터 킹까지** 네 개의 완성 더미를 만드세요.
sol_help_play = 게임 방법
sol_help_play_1 = • 열에서는 **빨강과 검정을 번갈아 내림차순**으로 놓습니다. 검정 7 위에 빨강 6처럼 놓으며 그 위의 카드도 함께 이동합니다.
sol_help_play_2 = • 빈 열에는 **킹**만 놓을 수 있습니다. 뒤집힌 카드를 가리던 카드가 없어지면 앞면이 됩니다.
sol_help_play_3 = • 덱을 탭해 버린 카드 더미로 넘기세요. 다 쓰면 다시 탭해 버린 카드를 덱으로 돌립니다.
sol_help_play_4 = • 카드를 **드래그**하거나 **탭**해 알맞은 곳으로 보냅니다. 모두 앞면이 되면 자동으로 완성 더미로 갑니다.
sol_help_score = 점수 계산
sol_help_score_1 = • 완성 더미로 보내면 10점, 버린 카드에서 열로 옮기면 5점, 뒤집으면 5점입니다. 완성 더미에서 꺼내면 15점 감점입니다.
sol_help_score_2 = • 버린 카드를 되돌리면 한 장 뽑기는 100점, 세 장 뽑기는 20점 감점입니다. 빠르게 이기면 보너스가 있습니다.
sol_help_winnable = 이길 수 있는 배치
sol_help_winnable_1 = • **이길 수 있는 배치만**을 켜면 미리 풀이를 확인합니다. **힌트**는 승리로 가는 수를 보여 주며, 불가능해졌다면 알려 줍니다.
sol_help_keys = 키보드
sol_help_keys_1 = • **1~7**로 열을 고르고 다른 숫자로 이동, 같은 숫자로 최적 위치에 보냅니다. **8**은 버린 카드, **9**는 완성 더미로, **0**은 덱 뒤집기입니다. 방향키로 위치와 카드 수를 조정합니다.

# Sudoku
su_difficulty = 난이도
su_time = 시간
su_easy = 쉬움
su_medium = 보통
su_hard = 어려움
su_expert = 전문가
su_detail_easy = 기본 숫자 46개 • 무제한 힌트
su_detail_medium = 기본 숫자 36개 • 힌트 3회
su_detail_hard = 기본 숫자 30개 • 힌트 없음
su_detail_expert = 기본 숫자 26개 • 힌트와 같은 숫자 강조 없음
su_cell_a11y = { $row }행 { $col }열
su_key_a11y = { $n } 입력
su_notes = 메모
su_notes_on = 메모 ✓
su_hint = 힌트
su_hint_n = 힌트 ({ $n })
su_hint_unlimited = 힌트 (∞)
su_undo = 실행 취소
su_redo = 다시 실행
su_checkpoint = 체크포인트
su_commit = 확정
su_revert = 되돌리기
su_give_up = 포기
su_give_up_title = 포기할까요?
su_give_up_message = 빈칸에 정답을 채우며 해결한 퍼즐로 계산하지 않습니다.
su_give_up_confirm = 포기
su_stars = ⭐⭐⭐
su_solved_title = 퍼즐 완성!
su_new_best = 최고 기록 갱신!
su_best = 최고: { $time }
su_choose_difficulty = 난이도 선택
su_default_difficulty = 기본 난이도
su_digit_font = 숫자 글꼴
su_records = 기록
su_puzzles_solved = 푼 퍼즐 수
su_reset_records = 스도쿠 기록 초기화
su_reset_title = 스도쿠 기록을 초기화할까요?
su_reset_message = 모든 스도쿠 최고 기록과 퍼즐 수가 영구적으로 초기화됩니다.

# Sudoku instructions (the how-to-play sheet; inline markdown for emphasis)
su_help_intro = 9×9 보드의 각 행, 열, 3×3 구역에 1~9가 한 번씩 들어가게 채우세요.
su_help_play = 게임 방법
su_help_play_1 = • **칸을 탭**해 선택하세요.
su_help_play_2 = • 숫자판의 **숫자를 탭**해 채우세요.
su_help_play_3 = • 이미 들어 있는 숫자의 키는 눌린 모양입니다. 다시 누르면 지워집니다.
su_help_play_4 = • 연필의 **메모 모드**로 가능한 숫자를 적으며 추리하세요.
su_help_play_5 = • 키보드로 숫자를 입력하거나 **0**으로 지우세요. 방향키로 선택을 옮깁니다.
su_help_checkpoint = 체크포인트
su_help_checkpoint_1 = • **체크포인트**로 시험 입력을 시작합니다. 이후의 숫자는 다른 색으로 표시되어 확정된 숫자와 구별됩니다.
su_help_checkpoint_2 = • 버튼이 **확정**(입력을 유지하고 강조 해제)과 **되돌리기**(시험 입력 모두 삭제)으로 나뉩니다.
su_help_undo = 실행 취소와 다시 실행
su_help_undo_1 = • **실행 취소**로 마지막 수를 되돌리면 **다시 실행**도 나타납니다.
su_help_undo_2 = • 현재 퍼즐에서는 횟수 제한이 없습니다. 취소 후 새 숫자를 넣으면 다시 실행 기록이 없어집니다.
su_help_win = 힌트와 승리
su_help_win_1 = • 처음 주어진 숫자는 바꿀 수 없습니다.
su_help_win_2 = • 플레이하는 동안 시간이 흐르며 난이도별 **최단 시간**이 저장됩니다.
su_help_win_3 = • 일부 난이도는 **힌트**를 제공합니다. 아껴 쓰세요.
su_help_win_4 = • 모든 칸을 올바르게 채우고 행, 열, 3×3 구역에 중복이 없으면 완료입니다.

# --- Charades ---
nav_charades = 몸으로 말해요
ch_pick_deck = 덱을 고르고 휴대폰을 이마에 대세요. 친구들이 힌트를 줍니다.
ch_deck_meta = 카드 { $count }장 · 최고 { $best }
ch_preview_word = 기린
ch_card_a11y = 카드: { $word }
ch_back = 뒤로
ch_start = 시작
ch_correct = 정답
ch_pass = 넘기기
ch_ready_title = 이마에 대세요
ch_ready_tilt = 화면은 바깥쪽으로. 가만히 들면 시작됩니다.
ch_ready_buttons = 친구들에게 화면을 보여 주고 시작을 누르세요.
ch_no_motion = 동작 센서가 응답하지 않아 이번 라운드는 버튼을 사용합니다.
ch_get_ready = 준비하세요
ch_correct_banner = 정답!
ch_pass_banner = 넘기기
ch_tilt_hint = 아래로 숙이면 정답 · 위로 젖히면 넘기기
ch_time_up = 시간 종료!
ch_out_of_cards = 카드를 모두 썼어요!
ch_new_best = 이 덱의 새 기록!
ch_play_again = 다시 하기
ch_decks = 덱
ch_end_round = 라운드 끝내기
ch_round_length = 라운드 길이
ch_seconds = { $n }초
ch_tilt = 기울여 답하기
ch_tilt_detail = 맞히면 아래로 끄덕이고 넘기려면 뒤로 젖히세요. 끄면 버튼을 사용합니다.
ch_tilt_missing = 동작 센서가 없는 기기입니다. 버튼을 사용하세요.
ch_records = 기록
ch_rounds_played = 플레이한 라운드
ch_reset_records = 기록 초기화
ch_reset_records_title = 기록을 초기화할까요?
ch_reset_records_message = 몸으로 말해요의 모든 최고 점수와 라운드 수를 영구적으로 삭제합니다.
ch_help_intro = 한 사람이 화면을 바깥으로 향해 이마에 대고 보지 않습니다. 다른 사람들은 화면의 단어를 맞힐 때까지 힌트를 줍니다.
ch_help_play = 게임 방법
ch_help_play_1 = • 덱을 고른 뒤 휴대폰을 이마에 대세요. 가만히 들면 시작합니다.
ch_help_play_2 = • 맞히면 **아래로 끄덕이고**, 넘기려면 **고개를 뒤로 젖히세요**.
ch_help_play_3 = • 설명, 몸짓, 허밍은 가능하지만 답을 말하거나 철자를 읽거나 가리키면 안 됩니다.
ch_help_play_4 = • 종료 후 맞힌 단어와 넘긴 단어를 확인하세요. 덱을 다 쓰기 전에는 단어가 반복되지 않습니다.
ch_help_buttons = 기울이지 않고 하기
ch_help_buttons_1 = • 센서가 없거나 **기울여 답하기**를 끄면 **정답**과 **넘기기**를 누르세요. 키보드는 ↓ 또는 Enter가 정답, ↑가 넘기기입니다.

# --- Mines ---
nav_mines = 지뢰찾기
mi_mines_left = 지뢰찾기
mi_time = 시간
mi_flag_mode = 깃발 모드
mi_flag_mode_a11y = 깃발 모드: 탭하면 깃발을 놓습니다
mi_board_a11y = 지뢰찾기 보드
mi_swept = 모두 찾았어요!
mi_boom = 펑!
mi_best_time = 최단 시간
mi_new_best = 최고 기록 갱신!
mi_try_again = 다시 시도
mi_easy = 쉬움
mi_medium = 보통
mi_hard = 어려움
mi_board_detail = { $cols } × { $rows } · 지뢰 { $mines }개
mi_choose = 보드 선택
mi_records = 기록
mi_record_line = { $played }판 중 { $won }승
mi_reset_records = 기록 초기화
mi_reset_records_title = 기록을 초기화할까요?
mi_reset_records_message = 지뢰찾기의 모든 최단 시간과 게임 수를 영구적으로 삭제합니다.
mi_help_intro = 보드에는 지뢰가 숨어 있습니다. 지뢰 없는 칸을 모두 열면 이깁니다.
mi_help_play = 게임 방법
mi_help_play_1 = • 탭해 여세요. **첫 탭은 항상 안전**하며 시작할 공간을 열어 줍니다.
mi_help_play_2 = • 숫자는 대각선을 포함한 주변 지뢰 수입니다. 빈칸은 주변도 함께 엽니다.
mi_help_play_3 = • **길게 눌러** 깃발을 놓거나 **깃발 모드**에서 탭하세요. 깃발을 길게 누르면 제거됩니다.
mi_help_play_4 = • 필요한 깃발이 모두 있는 숫자를 누르면 다른 이웃 칸을 엽니다. 깃발이 틀리면 위험합니다.
mi_help_keys = 키보드
mi_help_keys_1 = • 방향키로 이동하고 **Space**나 **Enter**로 열며 **F**로 깃발을 놓거나 지웁니다.

# Flipside
nav_flipside = 뒤집기
fs_black = 흑
fs_white = 백
fs_black_turn = 흑의 차례
fs_white_turn = 백의 차례
fs_thinking = 백이 생각 중…
fs_passed = 상대가 둘 수 없어 차례를 넘깁니다. { $turn }
fs_black_wins = 흑 승리!
fs_white_wins = 백 승리!
fs_draw = 무승부!
fs_final_score = 흑 { $black } · 백 { $white }
fs_board_hint = 점은 둘 수 있는 곳이며 금색 점은 마지막 수입니다.
fs_board_a11y = 뒤집기 보드. 방향키로 선택하고 Enter로 돌을 놓습니다.
fs_mode = 상대
fs_solo = 컴퓨터
fs_two_players = 두 명
fs_difficulty = 컴퓨터 난이도
fs_easy = 쉬움
fs_medium = 보통
fs_hard = 어려움
fs_start = 대국 시작
fs_help_rules = 흑이 먼저 둡니다. 점이 있는 곳에 놓아 내 돌 사이에 상대 돌의 줄을 가두세요. 가로, 세로, 대각선으로 갇힌 돌이 내 색으로 뒤집힙니다.
fs_help_pass = 둘 곳이 없으면 자동으로 차례를 넘깁니다. 둘 다 못 두면 빈칸이 있어도 끝나며 돌이 많은 쪽이 이깁니다.
fs_help_solo = 흑으로 컴퓨터와 겨루거나 새 게임에서 두 명을 고르세요. 쉬움은 무작위, 보통과 어려움은 앞을 내다봅니다. 모서리는 뒤집을 수 없으니 차지하세요!
fs_help_keys = 점을 탭하거나 방향키로 금색 테두리를 움직이고 Space나 Enter로 놓으세요. P나 Escape로 컴퓨터와 애니메이션도 일시 정지합니다. 나갈 때 저장됩니다.

fs_selection = { $column }{ $row }: { $state }
fs_legal = 가능한 수
fs_empty = 빈칸

# Pipes
nav_pipes = 파이프
pp_moves = 회전 수
pp_connected = 연결됨
pp_goal = 모든 파이프를 금색 수원에 연결하세요.
pp_solved = 모두 연결됐어요!
pp_lock_mode = 잠금 모드
pp_locked = 잠김
pp_unlocked = 잠금 해제
pp_north = 위
pp_east = 오른쪽
pp_south = 아래
pp_west = 왼쪽
pp_selection = { $row }행 { $column }열: { $ports }. { $state }.
pp_board_a11y = 파이프 보드. 방향키로 선택, Enter로 회전, L로 잠금 전환.
pp_size = 보드 크기
pp_small = 5 × 5
pp_medium = 7 × 7
pp_large = 9 × 9
pp_start = 퍼즐 시작
pp_restart = 퍼즐 다시 시작
pp_best = 최소 회전 수 · 이 크기
pp_help_goal = 타일을 돌려 모든 파이프를 중앙 금색 수원에 연결하세요. 연결된 파이프는 청록색으로 빛납니다. 모든 끝은 다른 파이프와 만나야 하며 보드 밖을 향하면 안 됩니다.
pp_help_controls = 탭하면 시계 방향으로 90도 회전합니다. 끝마개, 굽은 관, 직선과 분기가 하나의 망을 이룹니다. 모든 퍼즐은 풀 수 있으며 완성되면 빛의 물결이 퍼집니다.
pp_help_locks = 잠금 모드를 켜고 탭해 잠그거나 푸세요. 자물쇠가 있으면 회전하지 않습니다. 모드를 끄면 다시 회전할 수 있습니다. 다시 시작은 초기 배치로 돌아가 잠금과 횟수를 초기화합니다.
pp_help_keys = 방향키로 선택, Space나 Enter로 회전, L로 잠금, P나 Escape로 일시 정지합니다. 나가면 저장됩니다. 새 게임에서 5 × 5, 7 × 7, 9 × 9를 고르세요.

# The reason HarmonyOS shows when Charades asks for the accelerometer.
permission_ohos_permission_ACCELEROMETER = 몸으로 말해요는 휴대전화의 기울기를 감지하여 고개를 끄덕이면 정답으로 기록합니다.

## Match Three
nav_matchthree = 셋 맞추기
mt_moves = 남은 이동
mt_classic = 클래식
mt_corners = 모서리
mt_bridges = 다리
mt_diamond = 다이아몬드
mt_windows = 창문
mt_mixed = 혼합
mt_goals = 목표
mt_goal_score_name = 점수
mt_goal_collect_name = 빨간 원
mt_goal_frost_name = 얼음층
mt_goal_count = { $have } / { $need }
mt_goals_complete = 모든 목표 달성!
mt_still_needed = 아직 필요: { $list }
mt_score_not_enough = 점수 달성! 이기려면 아직 필요: { $list }
mt_need_points = { $n }점
mt_need_circles = 빨간 원 { $n }개
mt_need_layers = 얼음층 { $n }개
mt_moves_low = { $n ->
    [one] 마지막 이동!
   *[other] { $n }번만 남았어요!
}
mt_swap_hint = 이웃한 조각을 바꿔 셋 이상 맞추세요
mt_shuffling = 가능한 이동이 없습니다. 다시 섞는 중…
mt_cascade = 연쇄 ×{ $n }
mt_hint = 힌트
mt_levels = 레벨
mt_map_intro = 레벨을 완료하면 다음 레벨이 열립니다.
mt_locked = 잠김
mt_retry = 레벨 다시 하기
mt_reduced = 동작 줄이기
mt_theme = 테마
mt_theme_jewels = 보석처럼 반짝
mt_theme_stones = 바다 조약돌
mt_win = 레벨 완료
mt_complete = 모든 레벨 완료
mt_out = 이동 횟수 소진
mt_win_detail = 점수가 저장되었습니다. 준비되면 계속하세요.
mt_retry_detail = 다시 도전해 남은 목표를 완료하세요.
mt_next = 다음 레벨
mt_board_a11y = 셋 맞추기 보드. 방향키로 커서를 옮기고 Enter로 선택하거나 바꾸며 H로 가능한 교환을 표시합니다.
mt_help_swap = 보석을 길게 눌러 끌면 교환을 미리 볼 수 있습니다. 여러 이웃을 시도해 보고 손을 떼면 확정됩니다. 처음 칸으로 되돌리면 취소됩니다. 이웃한 두 조각을 차례로 탭해도 됩니다. 같은 모양을 셋 이상 맞추세요. 연쇄가 일어나면 점수가 올라갑니다. 맞춰지지 않는 교환은 이동으로 세지 않습니다.
mt_help_special_title = 특수 조각
mt_help_special = 넷을 맞추면 가로줄이나 세로줄을 없애는 줄무늬 조각이 생깁니다. T자나 L자로 맞추면 3 × 3 범위를 없애는 폭탄 조각이 생깁니다. 한 줄로 다섯을 맞추면 만능 조각이 생기며, 아무 조각과 바꾸면 그 색을 모두 없앱니다.
mt_help_combo = 특수 조각 둘을 합치면 더 넓게 없앱니다. 줄무늬 둘은 십자 모양으로, 폭탄 조합은 세 줄과 세 열을 없앱니다. 만능 조각과 특수 조각을 합치면 그 색의 모든 조각이 특수 조각이 됩니다. 만능 조각 둘은 보드 전체를 없앱니다.
mt_help_goals = 각 레벨의 목표는 점수 아래에 표시됩니다. 목표 점수에 도달하고, 이후 레벨에서는 빨간 원을 모으거나 조각 아래의 얼음층도 깨야 합니다. 이동 횟수가 끝나기 전에 모든 목표에 체크가 되어야만 이깁니다. 높은 점수만으로는 부족합니다. 이중 층은 두 번 맞혀야 합니다. 구멍은 보드를 서로 다른 낙하 레인으로 나눕니다. 10번 남기고 끝내면 별 3개, 5번이면 별 2개입니다. 완료한 레벨은 다시 할 수 있습니다.
mt_help_keys = 방향키와 Enter로 선택하고 바꿉니다. H는 가능한 교환을 표시하고 Enter로 실행합니다. 일시정지에서 설정, 규칙, 다시 하기를 열 수 있습니다. 효과음, 진동, 동작 줄이기를 조정할 수 있습니다. 보드와 진행 상황은 자동으로 저장됩니다.
mt_complete_detail = 레벨을 다시 플레이해 점수와 별점을 올릴 수 있습니다.
mt_matching = 맞추는 중…

# Local party games
nav_tictactoe = 틱택토
nav_memory = 기억력 게임
nav_moles = 두더지 잡기
party_ttt_rules = 같은 표시 세 개를 가로, 세로 또는 대각선으로 연결하세요. 플레이어 1은 ×, 플레이어 2는 ○입니다. 혼자 하면 컴퓨터가 상대가 됩니다.
party_memory_rules = 카드 두 장을 뒤집으세요. 같은 그림이면 한 쌍을 얻고 다시 플레이합니다. 다르면 차례가 넘어갑니다. 15쌍을 모두 찾아 가장 많이 모으면 승리합니다. 혼자 하면 컴퓨터가 상대가 됩니다.
party_moles_rules = 자기 색의 두더지를 누르세요. 모두 동시에 플레이하며 먼저 30점을 얻은 색이 이깁니다. 두더지 한 마리당 1점입니다. 혼자 하면 컴퓨터와 대결합니다.
party_controls = 칸을 누르거나 방향키로 선택한 뒤 Enter 또는 스페이스를 누르세요. Escape로 일시 정지합니다. 닫을 때 저장되며 다시 열면 일시 정지 상태입니다.
party_computer = 컴퓨터
party_player = 플레이어 { $n }
party_draw = 무승부!
party_wins = { $player } 승리!
party_turn = { $player } 차례
party_cross = 십자
party_ring = 원
party_empty = 빈칸
party_matched = 짝 찾음
party_card_value = 카드 { $n }
party_hidden = 뒷면
party_selection = { $row }행 { $column }열: { $state }
party_board = 게임판. 방향키로 선택하고 Enter로 플레이하세요.
party_count = { $n }

# Shared-device arcade games
nav_snakes = 뱀
nav_paintfight = 페인트 대결
nav_pool = 당구
arena_snakes_rules = 모서리의 조이스틱으로 조종하세요. 박자마다 몸이 길어지고 시간이 갈수록 성장 속도가 빨라집니다. 가장자리를 넘으면 반대편에 나타납니다. 다른 뱀의 머리와 몸을 피하세요. 자기 꼬리는 지나갈 수 있습니다. 마지막까지 살아남으면 승리합니다. 혼자서는 컴퓨터와 대결합니다.
arena_paint_rules = 45초 동안 상대보다 넓은 영역을 칠하세요. 자신의 조이스틱을 누르면 움직이고 놓으면 멈춥니다. 상대 색 위에 덧칠하고 번개를 모아 속도를 높이세요. 별을 얻으면 넓은 영역을 자신의 색으로 칠합니다. 가장 넓게 칠한 사람이 승리합니다. 혼자 하면 컴퓨터가 상대가 됩니다.
arena_pool_rules = 자기 그룹의 공 일곱 개를 넣고 마지막에 검은 8번 공을 넣으세요. 플레이어 1은 단색 공(1–7), 플레이어 2 또는 컴퓨터는 줄무늬 공(9–15)입니다. 8번 공을 먼저 넣으면 패배합니다. 자기 공을 넣으면 한 번 더 칩니다. 실패하거나 흰 공을 넣으면 차례가 넘어갑니다.
arena_pool_solids = 단색 · 1–7
arena_pool_stripes = 줄무늬 · 9–15
arena_controls = 각자 조이스틱을 동시에 드래그할 수 있습니다. 키보드: 플레이어 1 WASD, 2 방향키, 3 IJKL, 4 TFGH. 놓으면 현재 방향으로 계속 갑니다. Escape는 일시 정지입니다. 닫으면 저장되고 다시 열면 일시 정지 상태입니다.
arena_pool_controls = 흰 공에서 뒤로 드래그해 조준한 뒤 놓아 치세요. 길게 당길수록 힘이 강해집니다. 안내선이 방향을 표시합니다. 손가락, 마우스 또는 트랙패드를 사용하세요. 일시 정지에서 인원을 바꾸거나 다시 시작할 수 있습니다.
arena_countdown = { $n } 후 시작…
arena_remaining = { $seconds }초 남음
arena_rolling = 공이 움직이는 중…
arena_alive = 생존
arena_out = 탈락
arena_ai = AI
arena_board = 공유 경기장. 각자 모서리 조이스틱을 사용합니다. 당구에서는 흰 공에서 뒤로 드래그한 뒤 놓으세요.

# Mobile game catalog

party_moles_race = 먼저 30점 · 자기 색 누르기

party_round_wins = 라운드 승리
party_series_score = { $player }: { $wins }

# Shared player scoreboards
settings_general = 일반
party_turn_badge = 차례
party_playing_badge = 플레이 중
party_waiting_badge = 대기 중
party_winner_badge = 승자
party_finished_badge = 종료
party_everyone_plays = 모두 함께 플레이
party_wins_count = { $n }승
party_area_percent = { NUMBER($n, minimumFractionDigits: 1, maximumFractionDigits: 1) }%
party_status_detail = { $status } · { $detail }

home_player_count = 플레이어 수: { $n }
home_players_label = 인원:

nav_basketball = 농구 슛
basketball_swipe = 공을 골대 쪽으로 스와이프하세요
basketball_board = 농구 코트
basketball_rules = 공을 위로 스와이프해 슛하세요. 방향과 속도로 슛을 조절합니다. 플레이어마다 30초, 혼자서는 45초 동안 최대한 많이 득점하세요. 첫 슛부터 시간이 흐릅니다. 반대편 플레이어 차례에는 코트가 뒤집힙니다. 종료 전에 던진 공도 득점할 수 있습니다.
basketball_point = +1

# Reference party games
nav_buttonsoccer = 버튼 축구
nav_dotsboxes = 점과 상자
nav_impostor = 임포스터
nav_minigolf = 미니 골프
nav_fruitmerge = 과일 합치기
buttonsoccer_rules = 말을 뒤로 당겼다 놓아 슛하세요. 모든 말이 멈추면 차례가 바뀝니다. 먼저 세 골을 넣으면 승리합니다.
dotsboxes_rules = 두 점 사이의 빈 선을 누르세요. 상자를 완성하면 점수를 얻고 다시 플레이합니다. 상자를 가장 많이 차지하면 승리합니다.
impostor_rules = 기기를 돌려가며 자신의 단어를 몰래 확인하세요. 한 명은 임포스터입니다. 단서를 나누고 투표로 찾아내세요.
minigolf_rules = 공을 뒤로 당겼다 놓으세요. 번갈아 장애물의 반사도 이용해 홀을 노리세요. 일곱 라운드에서 가장 많이 성공하면 승리합니다.
fruitmerge_rules = 드래그로 조준하고 놓아 과일을 떨어뜨리세요. 같은 과일은 더 큰 과일로 합쳐집니다. 점선 아래로 쌓으세요.
fruitmerge_sweep = 정리

# Impostor: private reveal and group voting.
impostor_word_for = { $player }의 단어
impostor_hold = 길게 눌러 공개
impostor_role = 당신은 임포스터입니다!
impostor_discuss = 토론
impostor_clues = 비밀 단어와 관련된 단어나 문장을 차례로 말하세요. 시계 방향으로 1~3바퀴 돌고 누가 임포스터인지 토론하세요.
impostor_vote = 투표
impostor_choose = 누가 임포스터인지 함께 결정하세요.
impostor_confirm = { $player }에게 투표할까요?
impostor_caught = 임포스터 발견!
impostor_escaped = 임포스터 승리!
impostor_secret = 비밀 단어
impostor_pass = { $player }에게 넘기세요
impostor_hint = 임포스터에게 범주 표시
impostor_roles = 임포스터 수
impostor_categories = 범주
impostor_language = 단어 언어
impostor_next = 다음 라운드
impostor_content = { $text }

# Fruit Duel
nav_fruitduel = 과일 대결
fruitduel_rules = 과일이 나타나면 자신의 색 버튼을 누르세요. 먼저 과일을 자른 칼이 1점을 얻습니다. 선물, 꽃, 비치볼은 피하세요. 잘못 누르면 칼이 천천히 돌아옵니다. 먼저 10점을 얻으면 승리합니다. 혼자 컴퓨터와 대결하거나 최대 4명이 함께 즐기세요.

# Word-deck language names are shown in the language of the deck.
impostor_deck_english = English
impostor_deck_spanish = Español
impostor_deck_french = Français
impostor_deck_german = Deutsch
impostor_deck_italian = Italiano
impostor_deck_portuguese = Português

# Chess
nav_chess = 체스
chess_white = 백
chess_black = 흑
chess_turn = { $side } 차례
chess_check = { $side } 체크
chess_wins = { $side } 승리
chess_setup = 제한 시간 선택
chess_untimed = 시간 제한 없음
chess_blitz = 블리츠 · 5분
chess_rapid = 래피드 · 10분 + 수마다 5초
chess_help = 말을 눌러 가능한 수를 확인한 뒤 목적지를 누르거나 말을 끌어 놓으세요. 캐슬링은 킹을 두 칸 옮깁니다. 프로모션할 말을 선택할 수 있습니다. 체크메이트하면 승리합니다. 일시 정지 메뉴에서 무승부를 제안하거나 청구할 수 있습니다.
chess_claim = 무승부 청구
chess_intended = 무승부를 청구할 수를 선택하세요
chess_offer = 무승부 제안
chess_offer_question = 무승부를 수락할까요?
chess_accept = 수락
chess_decline = 거절
chess_resign = 기권
chess_promotion = 승격할 말 선택
chess_mate = 체크메이트
chess_stalemate = 스테일메이트
chess_dead = 메이트 불가능
chess_fifty = 50수 규칙
chess_three = 동일 국면 3회
chess_five = 동일 국면 5회
chess_seventyfive = 75수 규칙
chess_flag = 시간 초과
chess_agreed = 합의 무승부
chess_resigned = 기권
chess_pawn = 폰
chess_knight = 나이트
chess_bishop = 비숍
chess_rook = 룩
chess_queen = 퀸
chess_king = 킹
chess_empty = 빈칸
chess_thinking = 컴퓨터가 생각 중…
chess_flip = 매 수마다 보드 회전
chess_last = 마지막 수 · { $notation }
chess_square = { $square } · { $occupant }
chess_piece = { $side } { $kind }
chess_clock = { $minutes }:{ $digits }
chess_declined = 무승부 제안 거절
