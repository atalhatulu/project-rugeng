#!/usr/bin/env bash
set -e

# Project Rugeng - Baslatma ve Gelistirici Yonetim Scripti
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_ROOT"

# Renk Kodlari
C_RESET="\033[0m"
C_BOLD="\033[1m"
C_GREEN="\033[32m"
C_BLUE="\033[34m"
C_YELLOW="\033[33m"
C_RED="\033[31m"

print_header() {
    echo -e "${C_BLUE}${C_BOLD}=== Project Rugeng (Ember Engine) ===${C_RESET}"
}

usage() {
    print_header
    echo -e "Kullanim: ${C_GREEN}./start.sh${C_RESET} [komut] [secenekler]"
    echo ""
    echo "Komutlar:"
    echo -e "  ${C_BOLD}editor${C_RESET}       Editörü calistirir (varsayilan, release modunda)"
    echo -e "  ${C_BOLD}editor-dev${C_RESET}   Editörü gelistirici (debug) modunda calistirir"
    echo -e "  ${C_BOLD}bullet-hell${C_RESET}  Bullet Hell oyun prototipini calistirir"
    echo -e "  ${C_BOLD}platformer${C_RESET}   Platformer prototipini calistirir"
    echo -e "  ${C_BOLD}demo-3d${C_RESET}      3D demo sahnesini calistirir"
    echo -e "  ${C_BOLD}test${C_RESET}         Tum test paketini calistirir (--workspace)"
    echo -e "  ${C_BOLD}check${C_RESET}        Clippy, test ve format denetimini calistirir"
    echo -e "  ${C_BOLD}fmt${C_RESET}          Kodlari formatlar (cargo fmt)"
    echo -e "  ${C_BOLD}help${C_RESET}         Bu yardim metnini gosterir"
    echo ""
    echo "Ornekler:"
    echo "  ./start.sh                  # Release editörü baslatir"
    echo "  ./start.sh editor-dev       # Hizli derleme ile debug editörü baslatir"
    echo "  ./start.sh bullet-hell      # Bullet hell oyununu calistirir"
    echo "  ./start.sh check            # Kod kalitesi kontrolu yapar"
}

MODE="${1:-editor}"

case "$MODE" in
    editor)
        print_header
        echo -e "${C_GREEN}Editör calistiriliyor (Release modu)...${C_RESET}"
        cargo run --release -p rugeng_editor
        ;;
    editor-dev)
        print_header
        echo -e "${C_YELLOW}Editör calistiriliyor (Debug modu)...${C_RESET}"
        cargo run -p rugeng_editor
        ;;
    bullet-hell)
        print_header
        echo -e "${C_GREEN}Bullet Hell prototipi calistiriliyor (Release modu)...${C_RESET}"
        cargo run --release -p bullet_hell
        ;;
    platformer)
        print_header
        echo -e "${C_GREEN}Platformer prototipi calistiriliyor (Release modu)...${C_RESET}"
        cargo run --release -p platformer
        ;;
    demo-3d)
        print_header
        echo -e "${C_GREEN}3D Demo prototipi calistiriliyor (Release modu)...${C_RESET}"
        cargo run --release -p demo_3d
        ;;
    test)
        print_header
        echo -e "${C_BLUE}Tum testler calistiriliyor...${C_RESET}"
        cargo test --workspace
        ;;
    check)
        print_header
        echo -e "${C_BLUE}1/5: Testler denetleniyor (--workspace)...${C_RESET}"
        cargo test --workspace
        echo -e "${C_BLUE}2/5: Clippy analizi calistiriliyor...${C_RESET}"
        cargo clippy --workspace --all-targets -- -D warnings
        echo -e "${C_BLUE}3/5: Format uyumu kontrol ediliyor...${C_RESET}"
        cargo fmt --check
        echo -e "${C_BLUE}4/5: Loom eszamanlilik / veri yarisi analizi calistiriliyor...${C_RESET}"
        RUSTFLAGS="--cfg loom" cargo test --test loom_scheduler
        echo -e "${C_BLUE}5/5: Miri bellek guvenligi denetimi calistiriliyor...${C_RESET}"
        if command -v cargo-miri >/dev/null 2>&1 && rustup toolchain list | grep -q "nightly"; then
            cargo +nightly miri test -p rugeng_physics --lib
            cargo +nightly miri test -p rugeng_core --lib arena
        else
            echo -e "${C_YELLOW}Miri nightly araci bulunamadi, Miri adimi atlandi.${C_RESET}"
        fi
        echo -e "${C_GREEN}${C_BOLD}Tum denetimler basarili!${C_RESET}"
        ;;

    fmt)
        print_header
        echo -e "${C_BLUE}Kodlar formatlaniyor...${C_RESET}"
        cargo fmt
        echo -e "${C_GREEN}Formatlama tamamlandi.${C_RESET}"
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        echo -e "${C_RED}Gecersiz komut: '$MODE'${C_RESET}"
        usage
        exit 1
        ;;
esac
