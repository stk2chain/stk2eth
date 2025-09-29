import json
from pathlib import Path


def test_menu_contains_send_eth_service():
    menu_path = Path(__file__).parent.parent / "ussdgeth" / "src" / "data" / "menu.json"
    assert menu_path.exists(), f"menu.json not found at {menu_path}"
    menu = json.loads(menu_path.read_text())

    # service exists
    services = menu.get("services", {})
    assert "send_eth" in services, "send_eth service must be defined in services"

    # main menu option exists
    menus = menu.get("menus", {})
    main = menus.get("MainScreen")
    assert main is not None, "MainScreen must be present in menus"
    items = main.get("menu_items", {})
    found = False
    for _, item in items.items():
        if item.get("display_name") == "Send ETH" or item.get("next_screen") == "ToNumberScreen":
            found = True
            break
    assert found, "MainScreen must include a Send ETH menu item that points to ToNumberScreen"
