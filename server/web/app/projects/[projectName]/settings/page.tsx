"use client";

import { useState, useEffect } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

interface Project {
  name: string;
  owner_id: string;
  clone_url?: string;
}

interface User {
  id: string;
  name: string;
  email: string;
}

interface Permission {
  user_id: string;
  project: string;
  access: "read" | "write" | "admin";
}

interface SshKey {
  id: string;
  project: string;
  title: string;
  key_data: string;
  created_at: string;
}

export default function ProjectSettingsPage() {
  const params = useParams<{ projectName: string }>();
  const projectName = params.projectName;
  const router = useRouter();

  const [project, setProject] = useState<Project | null>(null);
  const [users, setUsers] = useState<User[]>([]);
  const [permissions, setPermissions] = useState<Permission[]>([]);
  const [sshKeys, setSshKeys] = useState<SshKey[]>([]);

  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [token, setToken] = useState("");

  const [activeTab, setActiveTab] = useState<"collaborators" | "keys">(
    "collaborators",
  );

  // State for setting new permissions
  const [permUserId, setPermUserId] = useState("");
  const [permAccess, setPermAccess] = useState("read");

  // State for new SSH keys
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

    fetch(`${API_BASE}/users/me`, {
      headers: { Authorization: `Bearer ${t}` },
    })
      .then((res) => {
        if (!res.ok) throw new Error("Auth failed");
        return res.json();
      })
      .then((data: User) => {
        setCurrentUser(data);
        fetchData(t, data.id);
      })
      .catch((error) => {
        console.error("Failed to fetch current user:", error);
        router.push("/auth/login");
      });
  }, [projectName, router]);

  const fetchData = async (authToken: string, currentUserId: string) => {
    try {
      const projectRes = await fetch(`${API_BASE}/projects/${projectName}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      if (!projectRes.ok) throw new Error("Project API Error");
      const projectData: Project = await projectRes.json();
      setProject(projectData);

      const [usersRes, permsRes, keysRes] = await Promise.all([
        fetch(`${API_BASE}/users`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
        fetch(`${API_BASE}/permissions`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
        fetch(`${API_BASE}/projects/${projectName}/ssh-keys`, {
          headers: { Authorization: `Bearer ${authToken}` },
        }),
      ]);

      const fetchedUsers = await usersRes.json();
      const fetchedPermissions = await permsRes.json();
      const fetchedKeys = await keysRes.json();

      setUsers(fetchedUsers);
      setPermissions(fetchedPermissions);
      setSshKeys(fetchedKeys);

      const userPermissions = fetchedPermissions.filter(
        (p: Permission) =>
          p.user_id === currentUserId && p.project === projectName,
      );
      const isAdminUser = userPermissions.some(
        (p: Permission) => p.access === "admin",
      );
      const isOwner = projectData.owner_id === currentUserId;
      setIsAdmin(isAdminUser || isOwner);

      if (!isAdminUser && !isOwner) {
        router.push(`/projects/${projectName}`);
      }
    } catch (e) {
      console.error(e);
      router.push("/auth/login");
    }
  };

  const handleSetPermission = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token || !projectName || !permUserId || !permAccess) return;

    try {
      const res = await fetch(`${API_BASE}/permissions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          user_id: permUserId,
          project: projectName,
          access: permAccess,
        }),
      });

      if (res.ok) {
        const permsRes = await fetch(`${API_BASE}/permissions`, {
          headers: { Authorization: `Bearer ${token}` },
        });
        setPermissions(await permsRes.json());
        setPermUserId("");
        setPermAccess("read");
      } else {
        const errorData = await res.json();
        alert(`Failed: ${errorData.message || res.statusText}`);
      }
    } catch (error) {
      console.error(error);
    }
  };

  const handlAddKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token || !projectName || !keyTitle || !keyData) return;
    setKeySubmitting(true);

    try {
      const res = await fetch(`${API_BASE}/projects/${projectName}/ssh-keys`, {
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
        const keysRes = await fetch(
          `${API_BASE}/projects/${projectName}/ssh-keys`,
          {
            headers: { Authorization: `Bearer ${token}` },
          },
        );
        setSshKeys(await keysRes.json());
        setKeyTitle("");
        setKeyData("");
      } else {
        alert("Failed to add SSH key. Note: Deploy keys must be unique.");
      }
    } catch (e) {
      console.error(e);
    } finally {
      setKeySubmitting(false);
    }
  };

  const handleDeleteKey = async (id: string) => {
    if (!confirm("Are you sure you want to delete this deploy key?")) return;

    try {
      const res = await fetch(
        `${API_BASE}/projects/${projectName}/ssh-keys/${id}`,
        {
          method: "DELETE",
          headers: { Authorization: `Bearer ${token}` },
        },
      );

      if (res.ok) {
        setSshKeys(sshKeys.filter((k) => k.id !== id));
      }
    } catch (e) {
      console.error(e);
    }
  };

  const assignableUsers = users.filter(
    (user) => user.id !== project?.owner_id && user.id !== currentUser?.id,
  );

  if (!project || !currentUser || !isAdmin) {
    return (
      <div className="flex min-h-[calc(100vh-56px)] justify-center bg-gray-50 py-20">
        <div className="h-10 w-10 animate-spin rounded-full border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="min-h-[calc(100vh-56px)] bg-gray-50">
      {/* Project Header (Gitea style) */}
      <div className="border-b border-gray-200 bg-white">
        <div className="mx-auto max-w-6xl px-4 py-5 sm:px-6 lg:px-8">
          <div className="mb-4 flex items-center space-x-2 text-xl">
            <div className="flex h-6 w-6 items-center justify-center rounded bg-indigo-100 text-xs font-bold text-indigo-700">
              {project.owner_id.charAt(0).toUpperCase()}
            </div>
            <span className="cursor-pointer font-medium text-gray-500 hover:text-indigo-600 hover:underline">
              {/* Note: we'd ideally show owner_name here instead of ID for realism */}
              User
            </span>
            <span className="text-gray-400">/</span>
            <Link
              href={`/projects/${projectName}`}
              className="font-bold text-gray-900 hover:text-indigo-600"
            >
              {project.name}
            </Link>
          </div>

          <nav className="flex space-x-6 text-sm font-medium">
            <Link
              href={`/projects/${projectName}`}
              className="border-b-2 border-transparent px-2 py-3 text-gray-500 hover:border-gray-300 hover:text-gray-700"
            >
              <span className="flex items-center gap-2">
                <svg
                  className="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
                  />
                </svg>
                Code
              </span>
            </Link>
            <Link
              href={`/projects/${projectName}/settings`}
              className="border-b-2 border-indigo-500 px-2 py-3 text-gray-900"
            >
              <span className="flex items-center gap-2">
                <svg
                  className="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                  />
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                  />
                </svg>
                Settings
              </span>
            </Link>
          </nav>
        </div>
      </div>

      <div className="mx-auto flex max-w-6xl gap-8 px-4 py-8 sm:px-6 lg:px-8">
        {/* Settings Sidebar */}
        <div className="w-1/4">
          <nav className="flex flex-col gap-1">
            <button
              onClick={() => setActiveTab("collaborators")}
              className={`rounded-md px-3 py-2 text-left text-sm font-medium transition-colors ${activeTab === "collaborators" ? "bg-indigo-50 text-indigo-700" : "text-gray-700 hover:bg-gray-100 hover:text-gray-900"}`}
            >
              Collaborators
            </button>
            <button
              onClick={() => setActiveTab("keys")}
              className={`rounded-md px-3 py-2 text-left text-sm font-medium transition-colors ${activeTab === "keys" ? "bg-indigo-50 text-indigo-700" : "text-gray-700 hover:bg-gray-100 hover:text-gray-900"}`}
            >
              Deploy Keys
            </button>
          </nav>
        </div>

        {/* Content Area */}
        <div className="w-3/4">
          {activeTab === "collaborators" && (
            <div>
              <div className="mb-6 border-b border-gray-200 pb-4">
                <h2 className="text-2xl font-semibold text-gray-900">
                  Manage Collaborators
                </h2>
                <p className="mt-1 text-sm text-gray-500">
                  Collaborators have read/write access to this repository based
                  on their roles.
                </p>
              </div>

              <div className="mb-8 bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
                <div className="rounded-t-lg border-b border-gray-200 bg-gray-50 px-4 py-3 sm:px-6">
                  <h3 className="text-base font-semibold text-gray-900">
                    Add Collaborator
                  </h3>
                </div>
                <div className="px-4 py-5 sm:p-6">
                  <form
                    onSubmit={handleSetPermission}
                    className="flex items-end gap-4"
                  >
                    <div className="flex-1">
                      <label className="mb-1 block text-sm font-medium text-gray-700">
                        Search user
                      </label>
                      <select
                        value={permUserId}
                        onChange={(e) => setPermUserId(e.target.value)}
                        required
                        className="block w-full rounded-md border-0 px-3 py-1.5 text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                      >
                        <option value="">Select a user...</option>
                        {assignableUsers.map((u) => (
                          <option key={u.id} value={u.id}>
                            {u.name} ({u.email})
                          </option>
                        ))}
                      </select>
                    </div>
                    <div className="w-32">
                      <label className="mb-1 block text-sm font-medium text-gray-700">
                        Permission
                      </label>
                      <select
                        value={permAccess}
                        onChange={(e) => setPermAccess(e.target.value)}
                        className="block w-full rounded-md border-0 px-3 py-1.5 text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                      >
                        <option value="read">Read</option>
                        <option value="write">Write</option>
                        <option value="admin">Admin</option>
                      </select>
                    </div>
                    <button
                      type="submit"
                      className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500"
                    >
                      Add
                    </button>
                  </form>
                </div>
              </div>

              <div className="overflow-hidden bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
                <ul role="list" className="divide-y divide-gray-100">
                  {users
                    .filter(
                      (user) =>
                        user.id !== project.owner_id &&
                        permissions.some(
                          (p) =>
                            p.user_id === user.id && p.project === projectName,
                        ),
                    )
                    .map((user) => {
                      const userPerm = permissions.find(
                        (p) =>
                          p.user_id === user.id && p.project === projectName,
                      );
                      const access = userPerm ? userPerm.access : "Unknown";
                      return (
                        <li
                          key={user.id}
                          className="flex items-center justify-between gap-x-6 px-4 py-5 hover:bg-gray-50 sm:px-6"
                        >
                          <div className="flex min-w-0 gap-x-4">
                            <div className="flex h-10 w-10 flex-none items-center justify-center rounded-full bg-gray-100 font-bold text-gray-500">
                              {user.name.charAt(0).toUpperCase()}
                            </div>
                            <div className="min-w-0 flex-auto">
                              <p className="text-sm leading-6 font-semibold text-gray-900">
                                {user.name}
                              </p>
                              <p className="mt-1 truncate text-xs leading-5 text-gray-500">
                                {user.email}
                              </p>
                            </div>
                          </div>
                          <div className="flex shrink-0 items-center gap-x-4">
                            <span className="inline-flex items-center rounded-md bg-blue-50 px-2 py-1 text-xs font-medium text-blue-700 capitalize ring-1 ring-blue-700/10 ring-inset">
                              {access}
                            </span>
                          </div>
                        </li>
                      );
                    })}

                  {users.filter(
                    (u) =>
                      u.id !== project.owner_id &&
                      permissions.some(
                        (p) => p.user_id === u.id && p.project === projectName,
                      ),
                  ).length === 0 && (
                    <li className="px-4 py-8 text-center text-sm text-gray-500">
                      No collaborators have been added yet
                    </li>
                  )}
                </ul>
              </div>
            </div>
          )}

          {activeTab === "keys" && (
            <div>
              <div className="mb-6 border-b border-gray-200 pb-4">
                <h2 className="text-2xl font-semibold text-gray-900">
                  Manage Deploy Keys
                </h2>
                <p className="mt-1 text-sm text-gray-500">
                  Deploy keys grant read-only access to this repository. They
                  are primarily used for CI/CD systems.
                </p>
              </div>

              <div className="mb-8 bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
                <div className="rounded-t-lg border-b border-gray-200 bg-gray-50 px-4 py-3 sm:px-6">
                  <h3 className="text-base font-semibold text-gray-900">
                    Add Deploy Key
                  </h3>
                </div>
                <div className="px-4 py-5 sm:p-6">
                  <form onSubmit={handlAddKey} className="space-y-4">
                    <div>
                      <label className="mb-1 block text-sm font-medium text-gray-700">
                        Title
                      </label>
                      <input
                        type="text"
                        required
                        value={keyTitle}
                        onChange={(e) => setKeyTitle(e.target.value)}
                        placeholder="e.g. Jenkins Deployment Key"
                        className="block w-full rounded-md border-0 px-3 py-1.5 text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                      />
                    </div>
                    <div>
                      <label className="mb-1 block text-sm font-medium text-gray-700">
                        Key Content
                      </label>
                      <textarea
                        required
                        rows={4}
                        value={keyData}
                        onChange={(e) => setKeyData(e.target.value)}
                        className="block w-full rounded-md border-0 px-3 py-1.5 font-mono text-gray-900 shadow-sm ring-1 ring-gray-300 ring-inset focus:ring-2 focus:ring-indigo-600 focus:ring-inset sm:text-sm sm:leading-6"
                        placeholder="Begins with 'ssh-rsa', 'ecdsa-sha2-nistp256', 'ecdsa-sha2-nistp384', 'ecdsa-sha2-nistp521', 'ssh-ed25519', etc"
                      />
                    </div>
                    <div className="text-sm text-gray-500">
                      We strongly recommend using Ed25519 keys for maximum
                      security.
                    </div>
                    <button
                      type="submit"
                      disabled={keySubmitting}
                      className={`rounded-md px-4 py-2 text-sm font-semibold text-white shadow-sm transition-all ${keySubmitting ? "cursor-not-allowed bg-green-400" : "bg-green-600 hover:bg-green-500"}`}
                    >
                      {keySubmitting ? "Adding..." : "Add Key"}
                    </button>
                  </form>
                </div>
              </div>

              <div className="overflow-hidden bg-white shadow ring-1 ring-gray-900/5 sm:rounded-lg">
                <ul role="list" className="divide-y divide-gray-100">
                  {sshKeys.map((key) => (
                    <li
                      key={key.id}
                      className="flex items-center justify-between gap-x-6 px-4 py-5 hover:bg-gray-50 sm:px-6"
                    >
                      <div className="flex min-w-0 items-center gap-x-4">
                        <svg
                          className="h-6 w-6 flex-none text-gray-400"
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
                        <div className="min-w-0 flex-auto">
                          <p className="text-sm leading-6 font-semibold text-gray-900">
                            {key.title}
                          </p>
                          <p className="mt-1 truncate font-mono text-xs leading-5 text-gray-500">
                            {key.key_data.substring(0, 40)}...
                          </p>
                        </div>
                      </div>
                      <div className="flex shrink-0 items-center gap-x-4">
                        <button
                          onClick={() => handleDeleteKey(key.id)}
                          className="rounded-md bg-red-50 px-3 py-1 text-sm font-medium text-red-600 transition-colors hover:bg-red-100 hover:text-red-500"
                        >
                          Delete
                        </button>
                      </div>
                    </li>
                  ))}

                  {sshKeys.length === 0 && (
                    <li className="border-t border-gray-100 px-4 py-8 text-center">
                      <div className="mx-auto h-12 w-12 text-gray-300">
                        <svg
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={1.5}
                            d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
                          />
                        </svg>
                      </div>
                      <h3 className="mt-2 text-sm font-medium text-gray-900">
                        No Deploy Keys
                      </h3>
                      <p className="mt-1 text-sm text-gray-500">
                        There are no deploy keys associated with this
                        repository.
                      </p>
                    </li>
                  )}
                </ul>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
