"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

interface User {
  id: string;
  name: string;
  email: string;
}

interface SshKey {
  id: string;
  user_id: string;
  title: string;
  key_data: string;
  created_at: string;
}

export default function UserSettingsPage() {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);
  const [sshKeys, setSshKeys] = useState<SshKey[]>([]);
  const [token, setToken] = useState("");

  const [keyTitle, setKeyTitle] = useState("");
  const [keyData, setKeyData] = useState("");
  const [keySubmitting, setKeySubmitting] = useState(false);

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (!t) {
      router.push("/auth/login");
      return;
    }
    setToken(t);
    fetchData(t);
  }, [router]);

  const fetchData = async (authToken: string) => {
    try {
      const [userRes, keysRes] = await Promise.all([
        fetch(`${API_BASE}/users/me`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
        fetch(`${API_BASE}/users/me/ssh-keys`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
      ]);

      if (userRes.ok) {
        setUser(await userRes.json());
      } else {
        router.push("/auth/login");
        return;
      }

      if (keysRes.ok) {
        setSshKeys(await keysRes.json());
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleAddKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token || !keyTitle || !keyData) return;
    setKeySubmitting(true);

    try {
      const res = await fetch(`${API_BASE}/users/me/ssh-keys`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          title: keyTitle,
          key_data: keyData,
        }),
      });

      if (res.ok) {
        const newKey = await res.json();
        setSshKeys([...sshKeys, newKey]);
        setKeyTitle("");
        setKeyData("");
      } else {
        const err = await res.json();
        alert(err.message || "Failed to add SSH key.");
      }
    } catch (e) {
      console.error(e);
    } finally {
      setKeySubmitting(false);
    }
  };

  const handleDeleteKey = async (id: string) => {
    if (!confirm("Are you sure you want to delete this SSH key?")) return;

    try {
      const res = await fetch(`${API_BASE}/users/me/ssh-keys/${id}`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${token}` },
      });

      if (res.ok) {
        setSshKeys(sshKeys.filter((k) => k.id !== id));
      }
    } catch (e) {
      console.error(e);
    }
  };

  if (!user) {
    return (
      <div className="flex min-h-[calc(100vh-56px)] justify-center bg-gray-50 py-20">
        <div className="h-10 w-10 animate-spin rounded-full border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="min-h-[calc(100vh-56px)] bg-gray-50">
      <div className="mx-auto max-w-4xl px-4 py-10 sm:px-6 lg:px-8">
        <div className="mb-8 border-b border-gray-200 pb-5">
          <h1 className="text-3xl font-bold text-gray-900">User Settings</h1>
          <p className="mt-2 text-sm text-gray-500">
            Manage your account settings and SSH keys.
          </p>
        </div>

        <div className="space-y-10">
          {/* Profile Section */}
          <section>
            <h2 className="mb-4 text-xl font-semibold text-gray-900">
              Profile
            </h2>
            <div className="bg-white p-6 shadow ring-1 ring-gray-900/5 sm:rounded-lg">
              <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Name
                  </label>
                  <p className="mt-1 text-sm text-gray-900">{user.name}</p>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Email
                  </label>
                  <p className="mt-1 text-sm text-gray-900">{user.email}</p>
                </div>
              </div>
            </div>
          </section>

          {/* SSH Keys Section */}
          <section>
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-xl font-semibold text-gray-900">SSH Keys</h2>
            </div>
            <p className="mb-4 text-sm text-gray-500">
              SSH keys allow you to securely clone and push to repositories
              without using a password.
            </p>

            <div className="mb-8 overflow-hidden bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
              <div className="border-b border-gray-200 bg-gray-50 px-4 py-3 sm:px-6">
                <h3 className="text-sm font-semibold text-gray-900">
                  Add SSH Key
                </h3>
              </div>
              <div className="p-6">
                <form onSubmit={handleAddKey} className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700">
                      Title
                    </label>
                    <input
                      type="text"
                      required
                      value={keyTitle}
                      onChange={(e) => setKeyTitle(e.target.value)}
                      placeholder="e.g. My Laptop"
                      className="mt-1 block w-full rounded-md border border-gray-300 p-2 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700">
                      Key
                    </label>
                    <textarea
                      required
                      rows={4}
                      value={keyData}
                      onChange={(e) => setKeyData(e.target.value)}
                      placeholder="Starts with 'ssh-rsa', 'ssh-ed25519', etc."
                      className="mt-1 block w-full rounded-md border border-gray-300 p-2 font-mono shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                    />
                  </div>
                  <button
                    type="submit"
                    disabled={keySubmitting}
                    className={`inline-flex justify-center rounded-md border border-transparent bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none ${keySubmitting ? "cursor-not-allowed opacity-50" : ""}`}
                  >
                    {keySubmitting ? "Adding..." : "Add SSH Key"}
                  </button>
                </form>
              </div>
            </div>

            <div className="overflow-hidden bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
              <ul role="list" className="divide-y divide-gray-200">
                {sshKeys.map((key) => (
                  <li
                    key={key.id}
                    className="flex items-center justify-between p-4 sm:px-6"
                  >
                    <div className="flex items-center gap-4">
                      <div className="flex-shrink-0">
                        <svg
                          className="h-6 w-6 text-gray-400"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
                          />
                        </svg>
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-gray-900">
                          {key.title}
                        </p>
                        <p className="max-w-md truncate font-mono text-xs text-gray-500">
                          {key.key_data.substring(0, 50)}...
                        </p>
                      </div>
                    </div>
                    <button
                      onClick={() => handleDeleteKey(key.id)}
                      className="ml-4 text-sm font-medium text-red-600 hover:text-red-500"
                    >
                      Delete
                    </button>
                  </li>
                ))}
                {sshKeys.length === 0 && (
                  <li className="p-10 text-center text-sm text-gray-500">
                    No SSH keys registered yet.
                  </li>
                )}
              </ul>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
