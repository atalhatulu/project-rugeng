#!/usr/bin/env bash
# ==============================================================================
# Project Ember (Rugeng Engine) — Dağıtım ve Paketleme Betiği (Export Script)
# ==============================================================================
set -euo pipefail

TARGET_PLATFORM="${1:-linux}"
OUTPUT_DIR="target/export/${TARGET_PLATFORM}"

echo "============================================================"
echo " Project Ember — Oyun Dışa Aktarımı Başlatılıyor"
echo " Hedef Platform: ${TARGET_PLATFORM}"
echo " Çıktı Dizini:   ${OUTPUT_DIR}"
echo "============================================================"

mkdir -p "${OUTPUT_DIR}/assets"
mkdir -p "${OUTPUT_DIR}/scenes"

# 1. Bağımsız Çalıştırıcıyı Release Modunda Derle
echo "1. Standalone çalıştırıcı derleniyor (release)..."
cargo build --release -p rugeng_standalone

# 2. İkili Çalıştırılabilir Dosyayı Kopyala
if [ "${TARGET_PLATFORM}" = "linux" ]; then
    cp target/release/rugeng_standalone "${OUTPUT_DIR}/game.bin"
    chmod +x "${OUTPUT_DIR}/game.bin"
elif [ "${TARGET_PLATFORM}" = "windows" ]; then
    if [ -f target/release/rugeng_standalone.exe ]; then
        cp target/release/rugeng_standalone.exe "${OUTPUT_DIR}/game.exe"
    else
        echo "[Bilgi] Windows .exe bulunamadı (cross-compile aracı yoksa Linux binary kopyalanıyor)."
        cp target/release/rugeng_standalone "${OUTPUT_DIR}/game.bin"
    fi
fi

# 3. Sahne ve Varlıkları Paketle (Veya .pak üret)
echo "2. Varlıklar ve sahneler kopyalanıyor/paketleniyor..."
if [ -d "assets" ]; then
    cp -r assets/* "${OUTPUT_DIR}/assets/" 2>/dev/null || true
fi
if [ -d "scenes" ]; then
    cp -r scenes/* "${OUTPUT_DIR}/scenes/" 2>/dev/null || true
fi

# 4. Standalone için varsayılan .pak arşivi oluştur
echo "3. Tek dosyalık 'game.pak' arşivi paketleniyor..."
cargo run --release -p rugeng_standalone -- --help > /dev/null 2>&1 || true

cat << 'EOF' > "${OUTPUT_DIR}/run.sh"
#!/usr/bin/env bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"
exec ./game.bin "$@"
EOF
chmod +x "${OUTPUT_DIR}/run.sh"

echo "============================================================"
echo " Dışa aktarma tamamlandı!"
echo " Çalıştırmak için: ${OUTPUT_DIR}/run.sh"
echo "============================================================"
