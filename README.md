# Rust Math

Ishxonadagi mahsulot uzunligini hisoblash uchun kichik Rust kalkulyator.

Kod Excel/Google Sheets qatoridagi quyidagi ustunlarga qarab uzunlik hisoblaydi:

- `KG`
- `RAZMER`
- `1 QAVAT`
- `1 MIKRON`
- `2 QAVAT`
- `2 MIKRON`

## Formula

Asosiy formula:

```text
uzunlik = KG / ((1-qavat koeffitsienti + 2-qavat koeffitsienti) * RAZMER_SM) * 6000
```

Keyin natijaga atxod qo'shiladi:

```text
atxod = uzunlik * 5%
yakuniy = uzunlik + atxod
```

Oxirida yakuniy uzunlik 500 ga yuqoriga yaxlitlanadi.

Masalan `1178` qator:

```text
KG = 300
RAZMER = 530 mm = 53 sm
1 QAVAT = pet, 1 MIKRON = 12  => 1
2 QAVAT = pe pr, 2 MIKRON = 30 => 2

1 + 2 = 3
3 * 53 = 159
300 / 159 * 6000 = 11321
11321 * 5% = 566
11321 + 566 = 11887
500 ga yuqoriga yaxlitlash = 12000
```

## Material Qoidalari

### 1-qavat

`pet`, `opp`, `popp`, `mat` materiallari 1-qavat oilasi hisoblanadi.

Agar ularning mikroni `20` yoki undan kichik bo'lsa, koeffitsient `1` olinadi.

```text
pet 12 => 1
mat 20 => 1
opp 18 => 1
```

Agar shu oiladagi material 20 dan katta mikronda kelsa, hozircha `MCP/CPP` jadvalidan olinadi.

### 2-qavat

2-qavat uchun koeffitsient material oilasi va mikron orqali jadvaldan olinadi.

`pe` bilan boshlanadigan materiallar `PE` oilasi:

```text
pe pr 30 => PE 30 => 2
pe oq 55 => PE 55 => 3.6
```

`cpp` yoki `mcp` bilan boshlanadigan materiallar `MCP/CPP` oilasi:

```text
cpp 45 => MCP/CPP 45 => 2.7
```

`oppm` hozir `opp` oilasi deb olinadi va 2-qavatda `MCP/CPP` jadvali orqali hisoblanadi:

```text
oppm 25/30 => 30 olinadi => MCP/CPP 30 => 1.6
oppm 20 => MCP/CPP 20 => 1.07
```

### Slash bilan yozilgan mikronlar

Agar material bitta bo'lib, mikron `18/20` yoki `25/30` shaklida yozilsa, kattasi olinadi:

```text
18/20 => 20
25/30 => 30
```

Agar material ham slash bilan yozilsa, u alohida qatlamlarga ajratiladi. Masalan `1207` qator:

```text
1 QAVAT = pet, 1 MIKRON = 12       => 1
2 QAVAT = oppm/pe pr, 2 MIKRON = 20/30

oppm 20  => MCP/CPP 20 => 1.07
pe pr 30 => PE 30      => 2

umumiy koeffitsient = 1 + 1.07 + 2 = 4.07
```

Bu holat aslida 3 qavat deb hisoblanadi.

### Twist / Tuisim

`twist` yoki `tuisim` odatda faqat 1-qavatga yoziladi va 2-qavati bo'lmaydi.

Bu holatda umumiy koeffitsient `2` deb olinadi:

```text
tuisim 23, 2-qavat -- => 2 + 0
```

## Koeffitsient Jadvali

### MCP/CPP

| Mikron | Koeffitsient |
|---:|---:|
| 20 | 1.07 |
| 25 | 1.3 |
| 30 | 1.6 |
| 35 | 2 |
| 40 | 2.15 |
| 45 | 2.7 |
| 50 | 2.8 |
| 60 | 3.2 |

### JEM

| Mikron | Koeffitsient |
|---:|---:|
| 25 | 1 |
| 30 | 1.5 |

### PE

| Mikron | Koeffitsient |
|---:|---:|
| 30 | 2 |
| 35 | 2.3 |
| 40 | 2.6 |
| 45 | 3 |
| 50 | 3.3 |
| 55 | 3.6 |
| 60 | 4 |
| 65 | 4.3 |
| 70 | 4.6 |
| 75 | 5 |
| 80 | 5.3 |
| 85 | 5.6 |
| 90 | 6 |

## Ishga Tushirish

Rust o'rnatilgan bo'lishi kerak.

```bash
cargo run
```

Terminalda savol-javob qilib hisoblash:

```bash
cargo run -- --interactive
```

Yoki qisqa:

```bash
make run
```

Interaktiv rejim siklda ishlaydi: bitta mahsulotni hisoblab bo'lgach yana yangi hisob boshlaydi. Chiqish uchun `Mahsulot og'irligi, kg` joyiga `q` yoziladi.

Dastur ketma-ket so'raydi:

- `Mahsulot og'irligi, kg`
- `RAZMER mm`
- `1-qavat material`
- `1-qavat mikron`
- `2-qavat material`
- `2-qavat mikron`
- `3-qavat material`
- `3-qavat mikron`
- `Atxod foizi`
- `Yaxlitlash`

3-qavat kerak bo'lmasa, `3-qavat material` joyini bo'sh qoldirib Enter bosiladi. 2-qavat ham bo'sh qoldirilsa, faqat 1-qavat bilan hisoblaydi.
`Atxod foizi [5]` va `Yaxlitlash [500]` joyida Enter bosilsa default qiymatlar olinadi.

Misol:

```text
KG: 3000
RAZMER mm: 635
1-qavat material: pet
1-qavat mikron: 12
2-qavat material: oppm
2-qavat mikron: 20
3-qavat material: pe pr
3-qavat mikron: 30
Atxod foizi [5]:
Yaxlitlash [500]:
```

Natija:

```text
yakuniy uzunlik: 73500
```

Parametr bilan:

```bash
cargo run -- --kg 300 --razmer 530 --q1 pet --m1 12 --q2 "pe pr" --m2 30
```

3-qavat parametr bilan:

```bash
cargo run -- --kg 3000 --razmer 635 --q1 pet --m1 12 --q2 oppm --m2 20 --q3 "pe pr" --m3 30
```

Natija:

```text
yakuniy uzunlik: 12000
```

## Faylni Hisoblash

Fayl berilganda dastur ustunlarni o'zi topadi va natijani input bilan bir xil formatda qaytaradi.

Hozir bir xil formatda qaytarish qo'llab-quvvatlanadi:

- `.xlsx -> .xlsx`
- `.csv -> .csv`
- `.tsv -> .tsv`

`.ods`, `.pdf`, `.html`, `.xls`, `.xlsm` formatlari taniladi, lekin hozircha ularni o'z formatida qayta yozish qo'shilmagan. Dastur bunday holatda aniq xato chiqaradi, jim konvert qilib yubormaydi.

Kerakli ustunlar:

- `KG`
- `RAZMER`
- `1 QAVAT`
- `1 MIKRON`
- `2 QAVAT`
- `2 MIKRON`

CSV fayl:

```bash
cargo run -- --file examples/sample.csv
```

TSV fayl:

```bash
cargo run -- --file ish.tsv
```

Excel fayl:

```bash
cargo run -- --file ish.xlsx
```

Output nomini o'zingiz berishingiz ham mumkin:

```bash
cargo run -- --file ish.xlsx --out natija.xlsx
```

Agar `--out` berilmasa, dastur fayl yoniga shunday output chiqaradi:

```text
ish_hisoblangan.xlsx
```

CSV bo'lsa:

```text
ish_hisoblangan.csv
```

Excel outputda dastur mavjud ustunlarga tegmaydi. Sheetdagi eng oxirgi ishlatilgan ustundan keyin `HISOBLANGAN_UZUNLIK` ustunini qo'shib, natijalarni o'sha yerga yozadi. CSV/TSV outputda esa oxiriga report ustunlari qo'shiladi. Xato qatorga `XATO: ...` deb yoziladi.

Outputga 3 ta ustun qo'shiladi:

- `HISOBLANGAN_UZUNLIK`
- `STATUS`
- `XATO`

Masalan:

```text
KOD,KG,RAZMER,1 QAVAT,1 MIKRON,2 QAVAT,2 MIKRON,HISOBLANGAN_UZUNLIK,STATUS,XATO
1178,300,530,pet,12,pe pr,30,12000,OK,
1207,3000,635,pet,12,oppm/pe pr,20/30,73500,OK,
```

Ustun nomlarida probel yoki tire farq qilmaydi. Masalan `1 QAVAT`, `1-qavat`, `1QAVAT` bir xil deb olinadi.

Material nomlarida ham oddiy imlo xatolarga chidamli:

```text
pett => pet
map  => mcp
twism => twist
pff => mat oilasi
petm/mpet => pet oilasi
pe pr, pe-pr, PE PR => pe pr
```

Bitta qavat bo'sh bo'lsa `--`, u 0 deb olinadi. Masalan 1-qavat bo'sh, 2-qavat `pe pr 60` bo'lsa faqat `PE 60` hisoblanadi.

## Telegram Bot

Bot alohida `bot/` papkada turadi. U admin private chatdan buyurtma ma'lumotlarini savol-javob qilib yig'adi:

- bitta guruhga screenshotdagi kabi rasm va chiroyli buyurtma matnini yuboradi;
- ikkinchi guruhga hisob-kitob va yakuniy uzunlikni yuboradi.

Kerakli env:

```bash
export BOT_TOKEN="telegram_bot_token"
```

Ishga tushirish:

```bash
make bot-run
```

Tekshirish:

```bash
make bot-check
```

Bot bilan ishlash:

```text
/new     yangi buyurtma boshlaydi
/cancel  joriy buyurtmani bekor qiladi
/login   guruh/chat rolini ulaydi
```

`/login` setup role ulash uchun ishlatiladi. Ma'lumot guruhi, hisob guruhi va admin chat alohida ulanadi. Login/parollar bot xabarlarida ko'rsatilmaydi.

Bot login va parol xabarlarini o'chiradi. Prompt xabarini `Login yozing` -> `Parol yozing` -> `Qabul qilindi: ...` qilib edit qiladi. Rolelar `bot_state.json`ga saqlanadi.

Buyurtma admin chatda bitta anketa xabarini edit qilib yig'iladi. Admin javoblari o'chiriladi, anketa esa to'ldirib boriladi. Bot quyidagilarni so'raydi: buyurtma raqami, mijoz, mahsulot, holat, material matni, rang, tiraj kg, uzunligi mm, eslatma va rasm. Material matnidan 1/2/3-qavat material va mikronlari avtomatik ajratiladi. Rasm yuborilsa ma'lumot guruhiga caption bilan tushadi, rasm bo'lmasa rasm bosqichida `-` yoziladi.

Admin chatga `.csv` yoki `.xlsx` fayl yuborilsa, bot uni hisoblab, shu formatda qaytaradi. Fayl ichida `KG`, `RAZMER`, `1 QAVAT`, `1 MIKRON`, `2 QAVAT`, `2 MIKRON` ustunlari bo'lishi kerak. Natijaga `HISOBLANGAN_UZUNLIK`, `STATUS`, `XATO` ustunlari qo'shiladi.

## Demo Qatorlar

Rasmdagi test qatorlarni hisoblatish:

```bash
cargo run -- --demo
```

Misol natijalar:

```text
1178    12000
1179    11500
1183    63500
1185    199000
1186    69000
1207    73500
```

`1185` twist/tuisim qatori 5% atxod bilan `199000` chiqadi. Agar amaldagi Excelda `195000` bo'lsa, twist uchun atxod/yaxlitlash qoidasi alohida aniqlanishi kerak.

## Test

```bash
cargo test
```

Yoki:

```bash
make test
```

Testlar quyidagilarni tekshiradi:

- `1178` qator `12000` chiqishi
- `pe pr` PE oilasi sifatida olinishi
- `pet` 20 dan katta bo'lsa `MCP/CPP` jadvaliga tushishi
- `tuisim` 2-qavatsiz hisoblanishi
- `oppm/pe pr` kabi 3-qavat yozuvlari alohida qatlamlarga ajratilishi

## Hozircha Aniqlashtirilishi Kerak Bo'lgan Joylar

- `twist/tuisim` uchun atxod har doim 5% bo'ladimi yoki boshqa qoida bormi?
- 2-qavat `--` bo'lgan boshqa materiallar qanday hisoblanadi?
- Excel faylni to'g'ridan-to'g'ri o'qish kerakmi yoki CSV orqali ishlash yetarlimi?
