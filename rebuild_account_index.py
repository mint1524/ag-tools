#!/usr/bin/env python3
"""
账号索引重建脚本
用于从现有账号文件重建 accounts.json 索引
"""

import json
import os
from pathlib import Path
from datetime import datetime

# 数据目录
DATA_DIR = Path.home() / ".antigravity_tools"
ACCOUNTS_DIR = DATA_DIR / "accounts"
INDEX_FILE = ACCOUNTS_DIR / "accounts.json"

def rebuild_index():
    """从现有账号文件重建索引"""
    
    if not ACCOUNTS_DIR.exists():
        print(f"❌ 账号目录不存在: {ACCOUNTS_DIR}")
        return False
    
    # 查找所有账号文件
    account_files = list(ACCOUNTS_DIR.glob("*.json"))
    account_files = [f for f in account_files if f.name != "accounts.json"]
    
    if not account_files:
        print("❌ 未找到任何账号文件")
        return False
    
    print(f"📁 找到 {len(account_files)} 个账号文件")
    
    # 读取每个账号文件并构建摘要
    accounts_summary = []
    current_account_id = None
    
    for account_file in sorted(account_files):
        try:
            with open(account_file, 'r', encoding='utf-8') as f:
                account = json.load(f)
            
            # 提取摘要信息
            summary = {
                "id": account.get("id"),
                "email": account.get("email"),
                "name": account.get("name"),
                "disabled": account.get("disabled", False),
                "proxy_disabled": account.get("proxy_disabled", False),
                "created_at": account.get("created_at", 0),
                "last_used": account.get("last_used", 0)
            }
            
            accounts_summary.append(summary)
            print(f"  ✓ {summary['email']} (ID: {summary['id'][:8]}...)")
            
        except Exception as e:
            print(f"  ✗ 读取失败: {account_file.name} - {e}")
    
    if not accounts_summary:
        print("❌ 没有成功读取任何账号")
        return False
    
    # 按 last_used 排序，最近使用的在前
    accounts_summary.sort(key=lambda x: x.get("last_used", 0), reverse=True)
    
    # 选择最近使用的账号作为当前账号
    if accounts_summary:
        current_account_id = accounts_summary[0]["id"]
    
    # 构建索引
    index = {
        "version": "2.0",
        "accounts": accounts_summary,
        "current_account_id": current_account_id
    }
    
    # 备份旧索引（如果存在）
    if INDEX_FILE.exists():
        backup_file = INDEX_FILE.with_suffix(f".json.backup.{int(datetime.now().timestamp())}")
        INDEX_FILE.rename(backup_file)
        print(f"📦 已备份旧索引: {backup_file.name}")
    
    # 写入新索引
    with open(INDEX_FILE, 'w', encoding='utf-8') as f:
        json.dump(index, f, indent=2, ensure_ascii=False)
    
    print(f"\n✅ 索引重建成功!")
    print(f"   - 总账号数: {len(accounts_summary)}")
    print(f"   - 当前账号: {accounts_summary[0]['email'] if accounts_summary else 'None'}")
    print(f"   - 索引文件: {INDEX_FILE}")
    
    return True

if __name__ == "__main__":
    print("🔧 开始重建账号索引...\n")
    success = rebuild_index()
    exit(0 if success else 1)
